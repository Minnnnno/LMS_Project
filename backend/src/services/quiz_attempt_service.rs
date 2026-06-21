use actix_session::Session;
use actix_web::HttpResponse;
use chrono::{Duration, Local};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Set, Statement, TransactionTrait,
};
use serde::Serialize;
use std::collections::HashSet;

use crate::entity::courses;
use crate::entity::quiz::{Column as QuizColumn, Entity as QuizEntity, Model as QuizModel};
use crate::entity::quiz_answers::{Column as QuizAnswerColumn, Entity as QuizAnswerEntity};
use crate::entity::quiz_attempts::{
    ActiveModel as QuizAttemptActiveModel, Column as QuizAttemptColumn, Entity as QuizAttemptEntity,
};
use crate::entity::quiz_options::{Column as QuizOptionColumn, Entity as QuizOptionEntity};
use crate::entity::quiz_questions::{
    Column as QuizQuestionColumn, Entity as QuizQuestionEntity, QuestionType,
};
use crate::entity::users::{Column as UserColumn, Entity as UserEntity};
use crate::models::quiz_attempts::CreateAttempt;
use crate::services::auth_helpers::{
    get_role_ids, get_user_id, has_staff_role, is_enrolled, is_student_only,
};
use crate::services::course_service::can_manage_course;
use crate::services::prerequisite_service;

#[derive(Serialize)]
struct QuizAttemptStatus {
    quiz_id: i32,
    attempts_used: usize,
    attempts_left: Option<i32>,
    max_attempts: Option<i32>,
    has_submitted_attempt: bool,
    can_attempt: bool,
    message: String,
}

#[derive(Serialize)]
struct AttemptTimer {
    time_limit_minutes: Option<i32>,
    expires_at: Option<String>,
    remaining_seconds: Option<i64>,
    message: String,
}

#[derive(Serialize)]
struct CreateAttemptResponse {
    attempt: crate::entity::quiz_attempts::Model,
    timer: AttemptTimer,
}

#[derive(Serialize)]
struct StaffQuizAttemptAnswer {
    answer_id: Option<i32>,
    question_id: i32,
    question_type: crate::entity::quiz_questions::QuestionType,
    question_text: String,
    points: i32,
    selected_option_id: Option<i32>,
    selected_option_text: Option<String>,
    correct_option_id: Option<i32>,
    correct_option_text: Option<String>,
    answer_text: Option<String>,
    score: Option<i32>,
    feedback: Option<String>,
}

#[derive(Serialize)]
struct StaffQuizAttempt {
    attempt_id: i32,
    quiz_id: i32,
    user_id: i32,
    student_name: String,
    student_email: String,
    started_at: chrono::NaiveDateTime,
    submitted_at: Option<chrono::NaiveDateTime>,
    total_score: Option<i32>,
    max_score: i32,
    is_graded: bool,
    answers: Vec<StaffQuizAttemptAnswer>,
}

#[derive(Serialize)]
struct StudentQuizAttemptReview {
    attempt_id: i32,
    quiz_id: i32,
    total_score: Option<i32>,
    max_score: i32,
    submitted_at: Option<chrono::NaiveDateTime>,
    answers: Vec<StaffQuizAttemptAnswer>,
}

fn require_staff(session: &Session, action: &str) -> Result<(), HttpResponse> {
    let role_ids = get_role_ids(session);
    if role_ids.is_empty() {
        return Err(HttpResponse::Unauthorized().body("You must be logged in"));
    }
    if is_student_only(&role_ids) {
        return Err(HttpResponse::Forbidden().body(format!("Students cannot {}", action)));
    }
    Ok(())
}

async fn require_can_manage_quiz(
    db: &DatabaseConnection,
    session: &Session,
    quiz: &QuizModel,
) -> Result<(), HttpResponse> {
    let course = courses::Entity::find_by_id(quiz.course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    match can_manage_course(db, session, &course).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(HttpResponse::Forbidden().body("You cannot view attempts for this quiz")),
        Err(response) => Err(response),
    }
}

fn build_attempt_timer(
    quiz: &QuizModel,
    attempt: Option<&crate::entity::quiz_attempts::Model>,
) -> AttemptTimer {
    let expires_at = quiz.time_limit.and_then(|minutes| {
        attempt.map(|attempt| {
            (attempt.started_at + Duration::minutes(minutes as i64))
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
        })
    });

    AttemptTimer {
        time_limit_minutes: quiz.time_limit,
        expires_at,
        remaining_seconds: quiz.time_limit.and_then(|minutes| {
            attempt.map(|attempt| {
                let expires_at = attempt.started_at + Duration::minutes(minutes as i64);
                (expires_at - Local::now().naive_local())
                    .num_seconds()
                    .max(0)
            })
        }),
        message: quiz
            .time_limit
            .map(|minutes| format!("{} minute time limit", minutes))
            .unwrap_or_else(|| "No time limit".to_string()),
    }
}

async fn finalize_attempt(
    db: &impl ConnectionTrait,
    attempt: crate::entity::quiz_attempts::Model,
) -> Result<(), HttpResponse> {
    let answers = QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt.attempt_id))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?;

    let has_short_answer_question = QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(attempt.quiz_id))
        .filter(QuizQuestionColumn::QuestionType.eq(QuestionType::LongAnswer))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?
        .into_iter()
        .next()
        .is_some();

    let total_score = answers
        .iter()
        .filter_map(|answer| answer.score)
        .sum::<i32>();
    let mut active: QuizAttemptActiveModel = attempt.into();
    active.submitted_at = Set(Some(Local::now().naive_local()));
    active.total_score = Set(Some(total_score));
    active.is_graded = Set(!has_short_answer_question);

    active
        .update(db)
        .await
        .map(|_| ())
        .map_err(|err| HttpResponse::InternalServerError().body(format!("Update error: {}", err)))
}
fn attempt_time_limit_expired(
    quiz: &QuizModel,
    attempt: &crate::entity::quiz_attempts::Model,
) -> bool {
    quiz.time_limit
        .map(|minutes| {
            Local::now().naive_local() >= attempt.started_at + Duration::minutes(minutes as i64)
        })
        .unwrap_or(false)
}

fn build_attempt_status(
    quiz_id: i32,
    max_attempts: Option<i32>,
    starts_at: Option<chrono::NaiveDateTime>,
    ends_at: Option<chrono::NaiveDateTime>,
    attempts: &[crate::entity::quiz_attempts::Model],
) -> QuizAttemptStatus {
    let attempts_used = attempts.len();
    let attempts_left = max_attempts.map(|max| (max - attempts_used as i32).max(0));
    let has_submitted_attempt = attempts
        .iter()
        .any(|attempt| attempt.submitted_at.is_some());

    if let Some(starts_at) = starts_at {
        if starts_at > Local::now().naive_local() {
            return QuizAttemptStatus {
                quiz_id,
                attempts_used,
                attempts_left,
                max_attempts,
                has_submitted_attempt,
                can_attempt: false,
                message: "This quiz is not open yet".to_string(),
            };
        }
    }

    if let Some(ends_at) = ends_at {
        if ends_at < Local::now().naive_local() {
            return QuizAttemptStatus {
                quiz_id,
                attempts_used,
                attempts_left,
                max_attempts,
                has_submitted_attempt,
                can_attempt: false,
                message: "This quiz is closed".to_string(),
            };
        }
    }

    if attempts_left == Some(0) {
        return QuizAttemptStatus {
            quiz_id,
            attempts_used,
            attempts_left,
            max_attempts,
            has_submitted_attempt,
            can_attempt: false,
            message: "No attempts left".to_string(),
        };
    }

    let message = match attempts_left {
        Some(1) => "1 attempt left".to_string(),
        Some(left) => format!("{} attempts left", left),
        None if has_submitted_attempt => "Attempted".to_string(),
        None => "Unlimited attempts".to_string(),
    };

    QuizAttemptStatus {
        quiz_id,
        attempts_used,
        attempts_left,
        max_attempts,
        has_submitted_attempt,
        can_attempt: true,
        message,
    }
}

async fn get_user_attempts_for_quiz(
    db: &impl ConnectionTrait,
    quiz_id: i32,
    user_id: i32,
) -> Result<Vec<crate::entity::quiz_attempts::Model>, HttpResponse> {
    QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::QuizId.eq(quiz_id))
        .filter(QuizAttemptColumn::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|err| HttpResponse::InternalServerError().body(format!("Database error: {}", err)))
}

async fn lock_attempt_creation(
    db: &impl ConnectionTrait,
    quiz_id: i32,
) -> Result<(), HttpResponse> {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1, $2)",
        [2.into(), quiz_id.into()],
    ))
    .await
    .map(|_| ())
    .map_err(|err| HttpResponse::InternalServerError().body(format!("Attempt lock error: {}", err)))
}

async fn lock_attempt(db: &impl ConnectionTrait, attempt_id: i32) -> Result<(), HttpResponse> {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1, $2)",
        [1.into(), attempt_id.into()],
    ))
    .await
    .map(|_| ())
    .map_err(|err| HttpResponse::InternalServerError().body(format!("Attempt lock error: {}", err)))
}

async fn ensure_quiz_is_attemptable(
    db: &impl ConnectionTrait,
    quiz_id: i32,
) -> Result<(), HttpResponse> {
    let questions = QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(quiz_id))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?;

    if questions.is_empty() {
        return Err(HttpResponse::Conflict().body("This quiz has no questions yet"));
    }

    for question in questions {
        if question.question_type != QuestionType::Mcq {
            continue;
        }

        let options = QuizOptionEntity::find()
            .filter(QuizOptionColumn::QuestionId.eq(question.question_id))
            .all(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!("Database error: {}", err))
            })?;

        if options.len() < 2 || !options.iter().any(|option| option.is_correct) {
            return Err(HttpResponse::Conflict()
                .body("This quiz has an invalid MCQ question and cannot be attempted yet"));
        }
    }

    Ok(())
}

async fn build_attempt_review_answers(
    db: &DatabaseConnection,
    quiz_id: i32,
    attempt_id: i32,
) -> Result<(i32, Vec<StaffQuizAttemptAnswer>), HttpResponse> {
    let questions = QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(quiz_id))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?;

    let mut questions = questions;
    questions.sort_by_key(|question| question.position);
    let max_score = questions
        .iter()
        .map(|question| question.points)
        .sum::<i32>();

    let answers = QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?;

    let mut review_answers = Vec::with_capacity(questions.len());

    for question in &questions {
        let answer = answers
            .iter()
            .find(|answer| answer.question_id == question.question_id);

        let selected_option = match answer.and_then(|answer| answer.selected_option_id) {
            Some(option_id) => QuizOptionEntity::find_by_id(option_id)
                .one(db)
                .await
                .map_err(|err| {
                    HttpResponse::InternalServerError().body(format!("Database error: {}", err))
                })?,
            None => None,
        };

        let correct_option = QuizOptionEntity::find()
            .filter(QuizOptionColumn::QuestionId.eq(question.question_id))
            .filter(QuizOptionColumn::IsCorrect.eq(true))
            .one(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!("Database error: {}", err))
            })?;

        review_answers.push(StaffQuizAttemptAnswer {
            answer_id: answer.map(|answer| answer.answer_id),
            question_id: question.question_id,
            question_type: question.question_type.clone(),
            question_text: question.question_text.clone(),
            points: question.points,
            selected_option_id: selected_option.as_ref().map(|option| option.option_id),
            selected_option_text: selected_option.map(|option| option.option_text),
            correct_option_id: correct_option.as_ref().map(|option| option.option_id),
            correct_option_text: correct_option.map(|option| option.option_text),
            answer_text: answer.and_then(|answer| answer.answer_text.clone()),
            score: answer.and_then(|answer| answer.score),
            feedback: answer.and_then(|answer| answer.feedback.clone()),
        });
    }

    Ok((max_score, review_answers))
}

pub async fn list_attempts_by_quiz(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> HttpResponse {
    if let Err(response) = require_staff(session, "view attempts by quiz") {
        return response;
    }

    let quiz = match QuizEntity::find_by_id(quiz_id).one(db).await {
        Ok(Some(quiz)) => quiz,
        Ok(None) => return HttpResponse::NotFound().body("Quiz not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    if let Err(response) = require_can_manage_quiz(db, session, &quiz).await {
        return response;
    }

    let questions = match QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(quiz_id))
        .all(db)
        .await
    {
        Ok(mut questions) => {
            questions.sort_by_key(|question| question.position);
            questions
        }
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    let max_score = questions
        .iter()
        .map(|question| question.points)
        .sum::<i32>();

    let attempts = match QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::QuizId.eq(quiz_id))
        .all(db)
        .await
    {
        Ok(mut attempts) => {
            attempts.sort_by(|first, second| second.started_at.cmp(&first.started_at));
            attempts
        }
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    let mut payload = Vec::with_capacity(attempts.len());

    for attempt in attempts {
        let student = match UserEntity::find()
            .filter(UserColumn::UserId.eq(attempt.user_id))
            .one(db)
            .await
        {
            Ok(Some(student)) => student,
            Ok(None) => return HttpResponse::NotFound().body("Student not found"),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error: {}", err));
            }
        };

        let answers = match QuizAnswerEntity::find()
            .filter(QuizAnswerColumn::AttemptId.eq(attempt.attempt_id))
            .all(db)
            .await
        {
            Ok(answers) => answers,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error: {}", err));
            }
        };

        let mut review_answers = Vec::with_capacity(questions.len());

        for question in &questions {
            let answer = answers
                .iter()
                .find(|answer| answer.question_id == question.question_id);

            let selected_option = match answer.and_then(|answer| answer.selected_option_id) {
                Some(option_id) => match QuizOptionEntity::find_by_id(option_id).one(db).await {
                    Ok(option) => option,
                    Err(err) => {
                        return HttpResponse::InternalServerError()
                            .body(format!("Database error: {}", err));
                    }
                },
                None => None,
            };

            let correct_option = match QuizOptionEntity::find()
                .filter(QuizOptionColumn::QuestionId.eq(question.question_id))
                .filter(QuizOptionColumn::IsCorrect.eq(true))
                .one(db)
                .await
            {
                Ok(option) => option,
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };

            review_answers.push(StaffQuizAttemptAnswer {
                answer_id: answer.map(|answer| answer.answer_id),
                question_id: question.question_id,
                question_type: question.question_type.clone(),
                question_text: question.question_text.clone(),
                points: question.points,
                selected_option_id: selected_option.as_ref().map(|option| option.option_id),
                selected_option_text: selected_option.map(|option| option.option_text),
                correct_option_id: correct_option.as_ref().map(|option| option.option_id),
                correct_option_text: correct_option.map(|option| option.option_text),
                answer_text: answer.and_then(|answer| answer.answer_text.clone()),
                score: answer.and_then(|answer| answer.score),
                feedback: answer.and_then(|answer| answer.feedback.clone()),
            });
        }

        payload.push(StaffQuizAttempt {
            attempt_id: attempt.attempt_id,
            quiz_id: attempt.quiz_id,
            user_id: attempt.user_id,
            student_name: format!("{} {}", student.first_name, student.last_name)
                .trim()
                .to_string(),
            student_email: student.email,
            started_at: attempt.started_at,
            submitted_at: attempt.submitted_at,
            total_score: attempt.total_score,
            max_score,
            is_graded: attempt.is_graded,
            answers: review_answers,
        });
    }

    HttpResponse::Ok().json(payload)
}

pub async fn list_my_attempts(db: &DatabaseConnection, session: &Session) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::UserId.eq(user_id))
        .all(db)
        .await
    {
        Ok(attempts) if attempts.is_empty() => HttpResponse::NotFound().body("No attempts found"),
        Ok(attempts) => HttpResponse::Ok().json(attempts),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn get_my_attempt_review(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let attempt = match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => attempt,
        Ok(None) => return HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    if attempt.user_id != user_id {
        return HttpResponse::Forbidden().body("You can only view your own quiz attempt");
    }

    if !attempt.is_graded {
        return HttpResponse::Forbidden().body("This quiz attempt has not been graded yet");
    }

    let (max_score, answers) =
        match build_attempt_review_answers(db, attempt.quiz_id, attempt.attempt_id).await {
            Ok(review) => review,
            Err(response) => return response,
        };

    HttpResponse::Ok().json(StudentQuizAttemptReview {
        attempt_id: attempt.attempt_id,
        quiz_id: attempt.quiz_id,
        total_score: attempt.total_score,
        max_score,
        submitted_at: attempt.submitted_at,
        answers,
    })
}

pub async fn list_my_attempt_statuses_by_course(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let quizzes = match QuizEntity::find()
        .filter(QuizColumn::CourseId.eq(course_id))
        .all(db)
        .await
    {
        Ok(quizzes) => quizzes,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    let mut statuses = Vec::with_capacity(quizzes.len());

    for quiz in quizzes {
        let attempts = match get_user_attempts_for_quiz(db, quiz.quiz_id, user_id).await {
            Ok(attempts) => attempts,
            Err(response) => return response,
        };

        let prerequisite_ids =
            match prerequisite_service::get_quiz_prerequisite_ids(db, quiz.quiz_id).await {
                Ok(ids) => ids,
                Err(response) => return response,
            };

        if let Some(prerequisite) =
            match prerequisite_service::get_first_incomplete_required_module(
                db,
                user_id,
                prerequisite_ids,
            )
            .await
            {
                Ok(module) => module,
                Err(response) => return response,
            }
        {
            statuses.push(QuizAttemptStatus {
                quiz_id: quiz.quiz_id,
                attempts_used: attempts.len(),
                attempts_left: quiz
                    .max_attempts
                    .map(|max| (max - attempts.len() as i32).max(0)),
                max_attempts: quiz.max_attempts,
                has_submitted_attempt: attempts
                    .iter()
                    .any(|attempt| attempt.submitted_at.is_some()),
                can_attempt: false,
                message: format!(
                    "Complete {} before attempting this quiz",
                    prerequisite.title
                ),
            });
            continue;
        }

        statuses.push(build_attempt_status(
            quiz.quiz_id,
            quiz.max_attempts,
            quiz.starts_at,
            quiz.ends_at,
            &attempts,
        ));
    }

    HttpResponse::Ok().json(statuses)
}

pub async fn create_attempt(
    db: &DatabaseConnection,
    session: &Session,
    data: CreateAttempt,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let role_ids = get_role_ids(session);

    if has_staff_role(&role_ids) {
        return HttpResponse::Forbidden()
            .body("Instructors and admins can view quiz questions but cannot attempt quizzes");
    }

    if !is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Student role required to attempt this quiz");
    }

    let quiz = match QuizEntity::find_by_id(data.quiz_id).one(db).await {
        Ok(Some(quiz)) => quiz,
        Ok(None) => return HttpResponse::NotFound().body("Quiz not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    match is_enrolled(db, user_id, quiz.course_id).await {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Forbidden().body("You must be enrolled to attempt this quiz");
        }
        Err(response) => return response,
    }

    if let Err(response) = ensure_quiz_is_attemptable(db, quiz.quiz_id).await {
        return response;
    }

    let prerequisite_ids =
        match prerequisite_service::get_quiz_prerequisite_ids(db, quiz.quiz_id).await {
            Ok(ids) => ids,
            Err(response) => return response,
        };

    match prerequisite_service::get_first_incomplete_required_module(db, user_id, prerequisite_ids)
        .await
    {
        Ok(Some(prerequisite)) => {
            return HttpResponse::Forbidden().body(format!(
                "Complete {} before attempting this quiz",
                prerequisite.title
            ));
        }
        Ok(None) => {}
        Err(response) => return response,
    }

    let transaction = match db.begin().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Could not start attempt: {}", err));
        }
    };
    if let Err(response) = lock_attempt_creation(&transaction, data.quiz_id).await {
        let _ = transaction.rollback().await;
        return response;
    }

    let quiz = match QuizEntity::find_by_id(data.quiz_id).one(&transaction).await {
        Ok(Some(quiz)) => quiz,
        Ok(None) => {
            let _ = transaction.rollback().await;
            return HttpResponse::NotFound().body("Quiz not found");
        }
        Err(err) => {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };
    if let Err(response) = ensure_quiz_is_attemptable(&transaction, quiz.quiz_id).await {
        let _ = transaction.rollback().await;
        return response;
    }
    let prerequisite_ids =
        match prerequisite_service::get_quiz_prerequisite_ids(&transaction, quiz.quiz_id).await {
            Ok(ids) => ids,
            Err(response) => {
                let _ = transaction.rollback().await;
                return response;
            }
        };
    match prerequisite_service::get_first_incomplete_required_module(
        &transaction,
        user_id,
        prerequisite_ids,
    )
    .await
    {
        Ok(Some(prerequisite)) => {
            let _ = transaction.rollback().await;
            return HttpResponse::Forbidden().body(format!(
                "Complete {} before attempting this quiz",
                prerequisite.title
            ));
        }
        Ok(None) => {}
        Err(response) => {
            let _ = transaction.rollback().await;
            return response;
        }
    }

    let mut attempts = match get_user_attempts_for_quiz(&transaction, data.quiz_id, user_id).await {
        Ok(attempts) => attempts,
        Err(response) => {
            let _ = transaction.rollback().await;
            return response;
        }
    };

    if let Some(open_attempt) = attempts
        .iter()
        .find(|attempt| attempt.submitted_at.is_none())
    {
        if attempt_time_limit_expired(&quiz, open_attempt) {
            if let Err(response) = lock_attempt(&transaction, open_attempt.attempt_id).await {
                let _ = transaction.rollback().await;
                return response;
            }
            if let Err(response) = finalize_attempt(&transaction, open_attempt.clone()).await {
                let _ = transaction.rollback().await;
                return response;
            }

            attempts = match get_user_attempts_for_quiz(&transaction, data.quiz_id, user_id).await {
                Ok(attempts) => attempts,
                Err(response) => {
                    let _ = transaction.rollback().await;
                    return response;
                }
            };
        } else {
            let response = CreateAttemptResponse {
                attempt: open_attempt.clone(),
                timer: build_attempt_timer(&quiz, Some(open_attempt)),
            };
            if let Err(err) = transaction.commit().await {
                return HttpResponse::InternalServerError()
                    .body(format!("Attempt transaction error: {}", err));
            }
            return HttpResponse::Ok().json(response);
        }
    }

    let status = build_attempt_status(
        quiz.quiz_id,
        quiz.max_attempts,
        quiz.starts_at,
        quiz.ends_at,
        &attempts,
    );

    if !status.can_attempt {
        let _ = transaction.rollback().await;
        return HttpResponse::Forbidden().body(status.message);
    }

    if let Some(max_attempts) = quiz.max_attempts {
        if attempts.len() >= max_attempts as usize {
            let _ = transaction.rollback().await;
            return HttpResponse::Forbidden().body("Maximum quiz attempts reached");
        }
    }

    let attempt = QuizAttemptActiveModel {
        quiz_id: Set(data.quiz_id),
        user_id: Set(user_id),
        started_at: Set(Local::now().naive_local()),
        is_graded: Set(false),
        ..Default::default()
    };

    match attempt.insert(&transaction).await {
        Ok(attempt) => {
            let response = CreateAttemptResponse {
                timer: build_attempt_timer(&quiz, Some(&attempt)),
                attempt,
            };
            match transaction.commit().await {
                Ok(_) => HttpResponse::Ok().json(response),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Attempt transaction error: {}", err)),
            }
        }
        Err(err) => {
            let _ = transaction.rollback().await;
            HttpResponse::InternalServerError().body(format!("Insert error: {}", err))
        }
    }
}

pub async fn submit_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let role_ids = get_role_ids(session);

    if has_staff_role(&role_ids) {
        return HttpResponse::Forbidden()
            .body("Instructors and admins can view quiz questions but cannot submit attempts");
    }

    if !is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Student role required to submit this quiz");
    }

    let transaction = match db.begin().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Could not start submission: {}", err));
        }
    };
    if let Err(response) = lock_attempt(&transaction, attempt_id).await {
        let _ = transaction.rollback().await;
        return response;
    }

    match QuizAttemptEntity::find_by_id(attempt_id)
        .one(&transaction)
        .await
    {
        Ok(Some(attempt)) => {
            if attempt.user_id != user_id {
                return HttpResponse::Forbidden().body("You can only submit your own attempt");
            }

            if attempt.submitted_at.is_some() {
                return HttpResponse::BadRequest().body("This attempt has already been submitted");
            }

            let quiz = match QuizEntity::find_by_id(attempt.quiz_id)
                .one(&transaction)
                .await
            {
                Ok(Some(quiz)) => quiz,
                Ok(None) => return HttpResponse::NotFound().body("Quiz not found"),
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };

            let time_limit_expired = attempt_time_limit_expired(&quiz, &attempt);

            let answers = match QuizAnswerEntity::find()
                .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
                .all(&transaction)
                .await
            {
                Ok(answers) => answers,
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };
            let questions = match QuizQuestionEntity::find()
                .filter(QuizQuestionColumn::QuizId.eq(attempt.quiz_id))
                .all(&transaction)
                .await
            {
                Ok(questions) => questions,
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };

            let answered_question_ids = answers
                .iter()
                .map(|answer| answer.question_id)
                .collect::<HashSet<i32>>();

            if !time_limit_expired
                && questions
                    .iter()
                    .any(|question| !answered_question_ids.contains(&question.question_id))
            {
                return HttpResponse::BadRequest()
                    .body("All questions must be answered before submission");
            }

            let total_score = answers
                .iter()
                .filter_map(|answer| answer.score)
                .sum::<i32>();
            let has_short_answer_question = questions.iter().any(|question| {
                question.question_type == crate::entity::quiz_questions::QuestionType::LongAnswer
            });
            let mut active: QuizAttemptActiveModel = attempt.into();
            active.submitted_at = Set(Some(Local::now().naive_local()));
            active.total_score = Set(Some(total_score));
            active.is_graded = Set(!has_short_answer_question);

            match active.update(&transaction).await {
                Ok(_) => match transaction.commit().await {
                    Ok(_) => HttpResponse::Ok().body(format!("Attempt {} submitted", attempt_id)),
                    Err(err) => HttpResponse::InternalServerError()
                        .body(format!("Submission transaction error: {}", err)),
                },
                Err(err) => {
                    let _ = transaction.rollback().await;
                    HttpResponse::InternalServerError().body(format!("Update error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn delete_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
) -> HttpResponse {
    if let Err(response) = require_staff(session, "delete attempts") {
        return response;
    }

    match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => {
            let quiz = match QuizEntity::find_by_id(attempt.quiz_id).one(db).await {
                Ok(Some(quiz)) => quiz,
                Ok(None) => return HttpResponse::NotFound().body("Quiz not found"),
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };
            if let Err(response) = require_can_manage_quiz(db, session, &quiz).await {
                return response;
            }

            let transaction = match db.begin().await {
                Ok(transaction) => transaction,
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Could not start attempt deletion: {}", err));
                }
            };
            if let Err(response) = lock_attempt(&transaction, attempt_id).await {
                let _ = transaction.rollback().await;
                return response;
            }
            let active_model: QuizAttemptActiveModel = attempt.into();
            match active_model.delete(&transaction).await {
                Ok(_) => match transaction.commit().await {
                    Ok(_) => HttpResponse::Ok().body("Attempt deleted!"),
                    Err(err) => HttpResponse::InternalServerError()
                        .body(format!("Attempt deletion transaction error: {}", err)),
                },
                Err(err) => {
                    let _ = transaction.rollback().await;
                    HttpResponse::InternalServerError().body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
