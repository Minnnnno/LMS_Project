use actix_session::Session;
use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::DatabaseConnection;

use crate::models::quiz::{CreateQuiz, UpdateQuiz};
use crate::services::quiz_service;

#[get("/quiz")]
pub async fn get_quiz(db: web::Data<DatabaseConnection>) -> impl Responder {
    quiz_service::list_quizzes(db.get_ref()).await
}

#[get("/quiz/{course_id}")]
pub async fn get_quiz_by_course_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_service::list_quizzes_by_course(db.get_ref(), path.into_inner()).await
}

#[get("/quiz/{quiz_id}/attempt-view")]
pub async fn get_quiz_attempt_view(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_service::get_quiz_for_attempt(db.get_ref(), &session, path.into_inner()).await
}

#[post("/quiz")]
pub async fn create_quiz(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateQuiz>,
) -> impl Responder {
    quiz_service::create_quiz(db.get_ref(), &session, body.into_inner()).await
}

#[put("/quiz/{quiz_id}")]
pub async fn update_quiz(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateQuiz>,
) -> impl Responder {
    quiz_service::update_quiz(db.get_ref(), &session, path.into_inner(), body.into_inner()).await
}

#[delete("/quiz/{quiz_id}")]
pub async fn delete_quiz(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_service::delete_quiz(db.get_ref(), &session, path.into_inner()).await
}
