use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::DatabaseConnection;

use crate::models::quiz_answers::{GradeQuizAnswer, SubmitLongAnswer, SubmitMcqAnswer};
use crate::services::quiz_answer_service;

#[get("/quiz-answers")]
pub async fn get_quiz_answers(db: web::Data<DatabaseConnection>) -> impl Responder {
    quiz_answer_service::list_answers(db.get_ref()).await
}

#[get("/quiz-answers/attempt/{attempt_id}")]
pub async fn get_answers_by_attempt_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_answer_service::list_answers_by_attempt(db.get_ref(), path.into_inner()).await
}

#[post("/quiz-answers/mcq")]
pub async fn submit_mcq_answer(
    db: web::Data<DatabaseConnection>,
    body: web::Json<SubmitMcqAnswer>,
) -> impl Responder {
    quiz_answer_service::submit_mcq_answer(db.get_ref(), body.into_inner()).await
}

#[post("/quiz-answers/long-answer")]
pub async fn submit_long_answer(
    db: web::Data<DatabaseConnection>,
    body: web::Json<SubmitLongAnswer>,
) -> impl Responder {
    quiz_answer_service::submit_long_answer(db.get_ref(), body.into_inner()).await
}

#[put("/quiz-answers/{answer_id}/grade")]
pub async fn grade_quiz_answer(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<GradeQuizAnswer>,
) -> impl Responder {
    quiz_answer_service::grade_answer(db.get_ref(), path.into_inner(), body.into_inner()).await
}

#[delete("/quiz-answers/{answer_id}")]
pub async fn delete_quiz_answer(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_answer_service::delete_answer(db.get_ref(), path.into_inner()).await
}
