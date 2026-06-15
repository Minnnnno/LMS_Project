use actix_session::Session;
use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::DatabaseConnection;

use crate::models::quiz_attempts::{CreateAttempt, MarkAttempt};
use crate::services::quiz_attempt_service;

// Staff only — see all attempts
#[get("/quiz-attempts")]
pub async fn get_quiz_attempts(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::list_attempts(db.get_ref(), &session).await
}

// Staff only — see all attempts for a quiz
#[get("/quiz-attempts/quiz/{quiz_id}")]
pub async fn get_attempts_by_quiz_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::list_attempts_by_quiz(db.get_ref(), &session, path.into_inner()).await
}

// Students see only their own attempts
#[get("/quiz-attempts/my")]
pub async fn get_my_attempts(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::list_my_attempts(db.get_ref(), &session).await
}

// Students see their own attempt availability for all quizzes in a course
#[get("/quiz-attempts/my/course/{course_id}/status")]
pub async fn get_my_attempt_statuses_by_course(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::list_my_attempt_statuses_by_course(
        db.get_ref(),
        &session,
        path.into_inner(),
    ).await
}

// Students create their own attempts; user_id is pulled from session, not the request body
#[post("/quiz-attempts")]
pub async fn create_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateAttempt>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::create_attempt(db.get_ref(), &session, body.into_inner()).await
}

// Students can only submit their own attempt
#[put("/quiz-attempts/{attempt_id}/submit")]
pub async fn submit_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::submit_attempt(db.get_ref(), &session, path.into_inner()).await
}

// Staff only — grade an attempt
#[put("/quiz-attempts/{attempt_id}/grade")]
pub async fn grade_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<MarkAttempt>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::grade_attempt(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    ).await
}

// Staff only — delete
#[delete("/quiz-attempts/{attempt_id}")]
pub async fn delete_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::delete_attempt(db.get_ref(), &session, path.into_inner()).await
}
