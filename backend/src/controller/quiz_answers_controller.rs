use actix_session::Session;
use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::DatabaseConnection;

use crate::models::quiz_answers::{SubmitMcqAnswer, SubmitLongAnswer, GradeQuizAnswer};
use crate::services::quiz_answer_service;

// GET /quiz-answers — staff only
#[get("/quiz-answers")]
pub async fn get_quiz_answers(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    quiz_answer_service::list_answers(db.get_ref(), &session).await
}

// GET /quiz-answers/attempt/{attempt_id}
// staff: see any attempt's answers
// students: only see answers belonging to their own attempt
#[get("/quiz-answers/attempt/{attempt_id}")]
pub async fn get_answers_by_attempt_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_answer_service::list_answers_by_attempt(db.get_ref(), &session, path.into_inner()).await
}

// POST /quiz-answers/mcq — logged in users (students submit their own attempt)
#[post("/quiz-answers/mcq")]
pub async fn submit_mcq_answer(
    db: web::Data<DatabaseConnection>,
    body: web::Json<SubmitMcqAnswer>,
    session: Session,
) -> impl Responder {
    quiz_answer_service::submit_mcq_answer(db.get_ref(), &session, body.into_inner()).await
}

// POST /quiz-answers/long-answer — logged in users
#[post("/quiz-answers/long-answer")]
pub async fn submit_long_answer(
    db: web::Data<DatabaseConnection>,
    body: web::Json<SubmitLongAnswer>,
    session: Session,
) -> impl Responder {
    quiz_answer_service::submit_long_answer(db.get_ref(), &session, body.into_inner()).await
}

// PUT /quiz-answers/{answer_id}/grade — staff only
#[put("/quiz-answers/{answer_id}/grade")]
pub async fn grade_quiz_answer(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<GradeQuizAnswer>,
    session: Session,
) -> impl Responder {
    quiz_answer_service::grade_answer(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    ).await
}

// DELETE /quiz-answers/{answer_id} — staff only
#[delete("/quiz-answers/{answer_id}")]
pub async fn delete_quiz_answer(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_answer_service::delete_answer(db.get_ref(), &session, path.into_inner()).await
}
