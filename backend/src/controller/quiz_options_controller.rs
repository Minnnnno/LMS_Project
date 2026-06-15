use actix_session::Session;
use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::DatabaseConnection;

use crate::models::quiz_options::{CreateQuizOption, UpdateQuizOption};
use crate::services::quiz_option_service;

#[get("/quiz-options")]
pub async fn get_quiz_options(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    quiz_option_service::list_options(db.get_ref(), &session).await
}

// Fixed: route param renamed from {option_id} to {question_id} to match intent
#[get("/quiz-options/by-question/{question_id}")]
pub async fn get_options_by_qn_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_option_service::list_options_by_question(db.get_ref(), &session, path.into_inner()).await
}

#[post("/quiz-options")]
pub async fn create_quiz_option(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateQuizOption>,
    session: Session,
) -> impl Responder {
    quiz_option_service::create_option(db.get_ref(), &session, body.into_inner()).await
}

#[put("/quiz-options/{option_id}")]
pub async fn update_quiz_option(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<UpdateQuizOption>,
    session: Session,
) -> impl Responder {
    quiz_option_service::update_option(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    ).await
}

#[delete("/quiz-options/{option_id}")]
pub async fn delete_quiz_option(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_option_service::delete_option(db.get_ref(), &session, path.into_inner()).await
}
