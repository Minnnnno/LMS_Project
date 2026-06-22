use actix_session::Session;
use actix_web::{Responder, delete, get, post, put, web};
use sea_orm::DatabaseConnection;

use crate::models::quiz::SaveQuizDraft;
use crate::services::quiz_service;

#[get("/quiz/{course_id}")]
pub async fn get_quiz_by_course_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_service::list_quizzes_by_course(db.get_ref(), path.into_inner()).await
}

#[post("/quiz/draft")]
pub async fn create_quiz_draft(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<SaveQuizDraft>,
) -> impl Responder {
    quiz_service::save_quiz_draft(db.get_ref(), &session, None, body.into_inner()).await
}

#[get("/quiz/{quiz_id}/draft")]
pub async fn get_quiz_draft(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_service::get_quiz_editor(db.get_ref(), &session, path.into_inner()).await
}

#[put("/quiz/{quiz_id}/draft")]
pub async fn update_quiz_draft(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<SaveQuizDraft>,
) -> impl Responder {
    quiz_service::save_quiz_draft(
        db.get_ref(),
        &session,
        Some(path.into_inner()),
        body.into_inner(),
    )
    .await
}

#[delete("/quiz/{quiz_id}")]
pub async fn delete_quiz(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    quiz_service::delete_quiz(db.get_ref(), &session, path.into_inner()).await
}
