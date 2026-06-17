use actix_session::Session;
use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::DatabaseConnection;

use crate::models::quiz_questions::{CreateQuizQuestion, UpdateQuizQuestion};
use crate::services::quiz_question_service;

#[get("/quiz-questions/{quiz_id}")]
pub async fn get_qns_by_quiz_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_question_service::list_questions_by_quiz(db.get_ref(), &session, path.into_inner()).await
}

#[post("/quiz-questions")]
pub async fn create_quiz_qn(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateQuizQuestion>,
    session: Session,
) -> impl Responder {
    quiz_question_service::create_question(db.get_ref(), &session, body.into_inner()).await
}

#[put("/quiz-questions/{question_id}")]
pub async fn update_quiz_qn(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<UpdateQuizQuestion>,
    session: Session,
) -> impl Responder {
    quiz_question_service::update_question(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    ).await
}

#[delete("/quiz-questions/{question_id}")]
pub async fn delete_quiz_qn(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_question_service::delete_question(db.get_ref(), &session, path.into_inner()).await
}
