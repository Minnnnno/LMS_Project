use actix_session::Session;
use actix_web::HttpResponse;
use chrono::Local;
use serde::Serialize;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::HashSet;

use crate::entity::quiz::{Column as QuizColumn, Entity as QuizEntity};
use crate::entity::quiz_answers::{Column as QuizAnswerColumn, Entity as QuizAnswerEntity};
use crate::entity::quiz_attempts::{
    ActiveModel as QuizAttemptActiveModel, Column as QuizAttemptColumn,
    Entity as QuizAttemptEntity,
};
use crate::entity::quiz_questions::{Column as QuizQuestionColumn, Entity as QuizQuestionEntity};
use crate::models::quiz_attempts::{CreateAttempt, MarkAttempt};
use crate::services::auth_helpers::{get_role_ids, get_user_id, is_enrolled, is_student_only};

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

fn build_attempt_status(
    quiz_id: i32,
    max_attempts: Option<i32>,
    starts_at: Option<chrono::NaiveDateTime>,
    attempts: &[crate::entity::quiz_attempts::Model],
) -> QuizAttemptStatus {
    let attempts_used = attempts.len();
    let attempts_left = max_attempts.map(|max| (max - attempts_used as i32).max(0));
    let has_submitted_attempt = attempts.iter().any(|attempt| attempt.submitted_at.is_some());

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
    db: &DatabaseConnection,
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

pub async fn list_attempts(db: &DatabaseConnection, session: &Session) -> HttpResponse {
    if let Err(response) = require_staff(session, "view all attempts") {
        return response;
    }

    match QuizAttemptEntity::find().all(db).await {
        Ok(attempts) if attempts.is_empty() => HttpResponse::NotFound().body("No quiz attempts found"),
        Ok(attempts) => HttpResponse::Ok().json(attempts),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_attempts_by_quiz(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> HttpResponse {
    if let Err(response) = require_staff(session, "view attempts by quiz") {
        return response;
    }

    match QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::QuizId.eq(quiz_id))
        .all(db)
        .await
    {
        Ok(attempts) if attempts.is_empty() => HttpResponse::NotFound().body("No attempts found"),
        Ok(attempts) => HttpResponse::Ok().json(attempts),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
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
        Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    };

    let mut statuses = Vec::with_capacity(quizzes.len());

    for quiz in quizzes {
        let attempts = match get_user_attempts_for_quiz(db, quiz.quiz_id, user_id).await {
            Ok(attempts) => attempts,
            Err(response) => return response,
        };

        statuses.push(build_attempt_status(
            quiz.quiz_id,
            quiz.max_attempts,
            quiz.starts_at,
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

    let quiz = match QuizEntity::find_by_id(data.quiz_id).one(db).await {
        Ok(Some(quiz)) => quiz,
        Ok(None) => return HttpResponse::NotFound().body("Quiz not found"),
        Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    };

    if is_student_only(&get_role_ids(session)) {
        match is_enrolled(db, user_id, quiz.course_id).await {
            Ok(true) => {}
            Ok(false) => return HttpResponse::Forbidden().body("You must be enrolled to attempt this quiz"),
            Err(response) => return response,
        }
    }

    let attempts = match get_user_attempts_for_quiz(db, data.quiz_id, user_id).await {
        Ok(attempts) => attempts,
        Err(response) => return response,
    };

    if let Some(open_attempt) = attempts.iter().find(|attempt| attempt.submitted_at.is_none()) {
        return HttpResponse::Ok().json(open_attempt);
    }

    let status = build_attempt_status(
        quiz.quiz_id,
        quiz.max_attempts,
        quiz.starts_at,
        &attempts,
    );

    if !status.can_attempt {
        return HttpResponse::Forbidden().body(status.message);
    }

    if let Some(max_attempts) = quiz.max_attempts {
        if attempts.len() >= max_attempts as usize {
            return HttpResponse::Forbidden().body("Maximum quiz attempts reached");
        }
    }

    let attempt = QuizAttemptActiveModel {
        quiz_id: Set(data.quiz_id),
        user_id: Set(user_id),
        ..Default::default()
    };

    match attempt.insert(db).await {
        Ok(attempt) => HttpResponse::Ok().json(attempt),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
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

    match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => {
            if attempt.user_id != user_id && is_student_only(&get_role_ids(session)) {
                return HttpResponse::Forbidden().body("You can only submit your own attempt");
            }

            if attempt.submitted_at.is_some() {
                return HttpResponse::BadRequest().body("This attempt has already been submitted");
            }

            let answers = match QuizAnswerEntity::find()
                .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
                .all(db)
                .await
            {
                Ok(answers) => answers,
                Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
            };
            let questions = match QuizQuestionEntity::find()
                .filter(QuizQuestionColumn::QuizId.eq(attempt.quiz_id))
                .all(db)
                .await
            {
                Ok(questions) => questions,
                Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
            };

            let answered_question_ids = answers
                .iter()
                .map(|answer| answer.question_id)
                .collect::<HashSet<i32>>();

            if questions
                .iter()
                .any(|question| !answered_question_ids.contains(&question.question_id))
            {
                return HttpResponse::BadRequest().body("All questions must be answered before submission");
            }

            let total_score = answers.iter().filter_map(|answer| answer.score).sum::<i32>();
            let mut active: QuizAttemptActiveModel = attempt.into();
            active.submitted_at = Set(Some(Local::now().naive_local()));
            active.total_score = Set(Some(total_score));

            match active.update(db).await {
                Ok(_) => HttpResponse::Ok().body(format!("Attempt {} submitted", attempt_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn grade_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
    data: MarkAttempt,
) -> HttpResponse {
    if let Err(response) = require_staff(session, "grade attempts") {
        return response;
    }

    match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => {
            let mut active: QuizAttemptActiveModel = attempt.into();

            if let Some(total_score) = data.total_score {
                active.total_score = Set(Some(total_score));
            }

            match active.update(db).await {
                Ok(_) => HttpResponse::Ok().body(format!("Score for attempt {} updated", attempt_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
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
            let active_model: QuizAttemptActiveModel = attempt.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Attempt deleted!"),
                Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
