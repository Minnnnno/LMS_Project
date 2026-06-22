use actix_session::Session;
use actix_web::{Responder, put, web};
use sea_orm::DatabaseConnection;

use crate::models::quiz_answers::{GradeQuizAnswer, SaveQuizAnswers};
use crate::services::quiz_answer_service;

#[put("/quiz-attempts/{attempt_id}/answers")]
pub async fn save_quiz_answers(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<SaveQuizAnswers>,
    session: Session,
) -> impl Responder {
    quiz_answer_service::save_answers(db.get_ref(), &session, path.into_inner(), body.into_inner())
        .await
}

#[put("/quiz-answers/{answer_id}/grade")]
pub async fn grade_quiz_answer(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<GradeQuizAnswer>,
    session: Session,
) -> impl Responder {
    quiz_answer_service::grade_answer(db.get_ref(), &session, path.into_inner(), body.into_inner())
        .await
}
