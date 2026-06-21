use actix_session::Session;
use actix_web::{Responder, get, web};
use sea_orm::DatabaseConnection;

use crate::services::quiz_analytics_service;

#[get("/quiz-analytics/course/{course_id}")]
pub async fn get_course_quiz_analytics(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_analytics_service::list_course_analytics(db.get_ref(), &session, path.into_inner()).await
}

#[get("/quiz-analytics/quiz/{quiz_id}")]
pub async fn get_quiz_analytics(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_analytics_service::get_quiz_analytics(db.get_ref(), &session, path.into_inner()).await
}
