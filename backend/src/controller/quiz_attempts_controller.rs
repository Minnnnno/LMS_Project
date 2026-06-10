use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::DatabaseConnection;

use crate::models::quiz_attempts::{CreateAttempt, MarkAttempt};
use crate::services::quiz_attempt_service;

#[get("/quiz-attempts")]
pub async fn get_quiz_attempts(db: web::Data<DatabaseConnection>) -> impl Responder {
    quiz_attempt_service::list_attempts(db.get_ref()).await
}

#[get("/quiz-attempts/{quiz_id}")]
pub async fn get_attempts_by_quiz_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_attempt_service::list_attempts_by_quiz(db.get_ref(), path.into_inner()).await
}

#[post("/quiz-attempts")]
pub async fn create_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateAttempt>,
) -> impl Responder {
    quiz_attempt_service::create_attempt(db.get_ref(), body.into_inner()).await
}

#[put("/quiz-attempts/{attempt_id}/submit")]
pub async fn submit_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_attempt_service::submit_attempt(db.get_ref(), path.into_inner()).await
}

#[put("/quiz-attempts/{attempt_id}/grade")]
pub async fn grade_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<MarkAttempt>,
) -> impl Responder {
    quiz_attempt_service::grade_attempt(db.get_ref(), path.into_inner(), body.into_inner()).await
}

#[delete("/quiz-attempts/{attempt_id}")]
pub async fn delete_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_attempt_service::delete_attempt(db.get_ref(), path.into_inner()).await
}
