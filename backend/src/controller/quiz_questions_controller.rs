use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::DatabaseConnection;

use crate::models::quiz_questions::{CreateQuizQuestion, UpdateQuizQuestion};
use crate::services::quiz_question_service;

#[get("/quiz-questions")]
pub async fn get_quiz_questions(db: web::Data<DatabaseConnection>) -> impl Responder {
    quiz_question_service::list_questions(db.get_ref()).await
}

#[get("/quiz-questions/{quiz_id}")]
pub async fn get_qns_by_quiz_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_question_service::list_questions_by_quiz(db.get_ref(), path.into_inner()).await
}

#[post("/quiz-questions")]
pub async fn create_quiz_qn(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateQuizQuestion>,
) -> impl Responder {
    quiz_question_service::create_question(db.get_ref(), body.into_inner()).await
}

#[put("/quiz-questions/{question_id}")]
pub async fn update_quiz_qn(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<UpdateQuizQuestion>,
) -> impl Responder {
    quiz_question_service::update_question(db.get_ref(), path.into_inner(), body.into_inner()).await
}

#[delete("/quiz-questions/{question_id}")]
pub async fn delete_quiz_qn(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_question_service::delete_question(db.get_ref(), path.into_inner()).await
}
