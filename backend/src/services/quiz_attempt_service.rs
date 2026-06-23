use actix_session::Session;
use actix_web::HttpResponse;
use chrono::Local;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set, TransactionTrait,
};
use serde::Serialize;
use std::collections::HashSet;

use crate::entity::quiz::{Column as QuizColumn, Entity as QuizEntity, Model as QuizModel};
use crate::entity::quiz_answers::{Column as QuizAnswerColumn, Entity as QuizAnswerEntity};
use crate::entity::quiz_attempts::{
    ActiveModel as QuizAttemptActiveModel, Column as QuizAttemptColumn, Entity as QuizAttemptEntity,
};
use crate::entity::quiz_questions::{
    Column as QuizQuestionColumn, Entity as QuizQuestionEntity, QuestionType,
};
use crate::models::quiz_answers::SavedQuizAnswer;
use crate::models::quiz_attempts::{AttemptAccess, AttemptTimer, StartAttemptResponse};
use crate::services::auth_helpers::{
    get_role_ids, get_user_id, has_staff_role, is_enrolled, is_student_only,
};
use crate::services::prerequisite_service;
use crate::services::quiz_helper;

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

fn build_attempt_timer(
    quiz: &QuizModel,
    attempt: Option<&crate::entity::quiz_attempts::Model>,
) -> AttemptTimer {
    let expires_at = attempt
        .and_then(|attempt| quiz_helper::attempt_expires_at(quiz.time_limit, attempt.started_at))
        .map(|expires_at| expires_at.format("%Y-%m-%dT%H:%M:%S").to_string());

    AttemptTimer {
        time_limit_minutes: quiz.time_limit,
        expires_at,
        remaining_seconds: attempt.and_then(|attempt| {
            quiz_helper::attempt_expires_at(quiz.time_limit, attempt.started_at).map(|expires_at| {
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

async fn build_start_response(
    db: &impl ConnectionTrait,
    quiz: QuizModel,
    attempt: Option<crate::entity::quiz_attempts::Model>,
    access: AttemptAccess,
) -> Result<StartAttemptResponse, HttpResponse> {
    let questions = quiz_helper::load_attempt_questions(db, quiz.quiz_id).await?;
    let answers = if let Some(attempt) = attempt.as_ref() {
        QuizAnswerEntity::find()
            .filter(QuizAnswerColumn::AttemptId.eq(attempt.attempt_id))
            .all(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!("Database error: {}", err))
            })?
            .into_iter()
            .map(|answer| SavedQuizAnswer {
                question_id: answer.question_id,
                selected_option_id: answer.selected_option_id,
                answer_text: answer.answer_text,
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(StartAttemptResponse {
        timer: build_attempt_timer(&quiz, attempt.as_ref()),
        quiz,
        questions,
        access,
        attempt,
        answers,
    })
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

    let short_answer_question_ids = QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(attempt.quiz_id))
        .filter(QuizQuestionColumn::QuestionType.eq(QuestionType::LongAnswer))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?
        .into_iter()
        .map(|question| question.question_id)
        .collect::<HashSet<_>>();
    let requires_manual_grading = answers
        .iter()
        .any(|answer| short_answer_question_ids.contains(&answer.question_id));

    let total_score = answers
        .iter()
        .filter_map(|answer| answer.score)
        .sum::<i32>();
    let mut active: QuizAttemptActiveModel = attempt.into();
    active.submitted_at = Set(Some(Local::now().naive_local()));
    active.total_score = Set(Some(total_score));
    active.is_graded = Set(!requires_manual_grading);

    active
        .update(db)
        .await
        .map(|_| ())
        .map_err(|err| HttpResponse::InternalServerError().body(format!("Update error: {}", err)))
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
    quiz_id: i32,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let role_ids = get_role_ids(session);

    let quiz = match QuizEntity::find_by_id(quiz_id).one(db).await {
        Ok(Some(quiz)) => quiz,
        Ok(None) => return HttpResponse::NotFound().body("Quiz not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    if has_staff_role(&role_ids) {
        if let Err(response) = quiz_helper::require_can_manage_quiz(db, session, quiz.quiz_id).await
        {
            return response;
        }
        return match build_start_response(
            db,
            quiz,
            None,
            AttemptAccess {
                can_attempt: false,
                preview_only: true,
                message:
                    "You can view the questions, but instructors and admins cannot attempt quizzes."
                        .to_string(),
            },
        )
        .await
        {
            Ok(response) => HttpResponse::Ok().json(response),
            Err(response) => response,
        };
    }

    if !is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Student role required to attempt this quiz");
    }

    match is_enrolled(db, user_id, quiz.course_id).await {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Forbidden().body("You must be enrolled to attempt this quiz");
        }
        Err(response) => return response,
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
    if let Err(response) = quiz_helper::lock_quiz(&transaction, quiz_id).await {
        let _ = transaction.rollback().await;
        return response;
    }

    let quiz = match QuizEntity::find_by_id(quiz_id).one(&transaction).await {
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

    let mut attempts = match get_user_attempts_for_quiz(&transaction, quiz_id, user_id).await {
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
        if quiz_helper::attempt_time_limit_expired(quiz.time_limit, open_attempt.started_at) {
            if let Err(response) =
                quiz_helper::lock_attempt(&transaction, open_attempt.attempt_id).await
            {
                let _ = transaction.rollback().await;
                return response;
            }
            if let Err(response) = finalize_attempt(&transaction, open_attempt.clone()).await {
                let _ = transaction.rollback().await;
                return response;
            }

            attempts = match get_user_attempts_for_quiz(&transaction, quiz_id, user_id).await {
                Ok(attempts) => attempts,
                Err(response) => {
                    let _ = transaction.rollback().await;
                    return response;
                }
            };
        } else {
            let response = match build_start_response(
                &transaction,
                quiz,
                Some(open_attempt.clone()),
                AttemptAccess {
                    can_attempt: true,
                    preview_only: false,
                    message: "You can attempt this quiz.".to_string(),
                },
            )
            .await
            {
                Ok(response) => response,
                Err(response) => {
                    let _ = transaction.rollback().await;
                    return response;
                }
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
        quiz_id: Set(quiz_id),
        user_id: Set(user_id),
        started_at: Set(Local::now().naive_local()),
        is_graded: Set(false),
        ..Default::default()
    };

    match attempt.insert(&transaction).await {
        Ok(attempt) => {
            let response = match build_start_response(
                &transaction,
                quiz,
                Some(attempt),
                AttemptAccess {
                    can_attempt: true,
                    preview_only: false,
                    message: "You can attempt this quiz.".to_string(),
                },
            )
            .await
            {
                Ok(response) => response,
                Err(response) => {
                    let _ = transaction.rollback().await;
                    return response;
                }
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
    if let Err(response) = quiz_helper::lock_attempt(&transaction, attempt_id).await {
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

            let time_limit_expired =
                quiz_helper::attempt_time_limit_expired(quiz.time_limit, attempt.started_at);

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

            match finalize_attempt(&transaction, attempt).await {
                Ok(()) => match transaction.commit().await {
                    Ok(_) => HttpResponse::Ok().body(format!("Attempt {} submitted", attempt_id)),
                    Err(err) => HttpResponse::InternalServerError()
                        .body(format!("Submission transaction error: {}", err)),
                },
                Err(err) => {
                    let _ = transaction.rollback().await;
                    err
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
    if let Err(response) = quiz_helper::require_staff(session) {
        return response;
    }

    let attempt = match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => attempt,
        Ok(None) => return HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    if let Err(response) = quiz_helper::require_can_manage_quiz(db, session, attempt.quiz_id).await {
        return response;
    }

    let transaction = match db.begin().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Could not start attempt delete: {}", err));
        }
    };

    if let Err(response) = quiz_helper::lock_attempt(&transaction, attempt_id).await {
        let _ = transaction.rollback().await;
        return response;
    }

    match QuizAnswerEntity::delete_many()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .exec(&transaction)
        .await
    {
        Ok(_) => {}
        Err(err) => {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError()
                .body(format!("Attempt answer delete error: {}", err));
        }
    }

    match QuizAttemptEntity::delete_by_id(attempt_id)
        .exec(&transaction)
        .await
    {
        Ok(result) if result.rows_affected > 0 => {}
        Ok(_) => {
            let _ = transaction.rollback().await;
            return HttpResponse::NotFound().body("Attempt not found");
        }
        Err(err) => {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError()
                .body(format!("Attempt delete error: {}", err));
        }
    }

    match transaction.commit().await {
        Ok(_) => HttpResponse::Ok().body("Quiz attempt deleted"),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Attempt delete commit error: {}", err))
        }
    }
}
