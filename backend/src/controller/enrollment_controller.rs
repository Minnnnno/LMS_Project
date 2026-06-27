use actix_session::Session;
use actix_web::{Responder, get, post, web};
use sea_orm::DatabaseConnection;

use crate::services::enrollment_service;

#[post("/courses/{course_id}/enroll")]
pub async fn enroll_free_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    enrollment_service::enroll_free_course(db.get_ref(), &session, path.into_inner()).await
}

#[get("/courses/{course_id}/enrollment-status")]
pub async fn get_enrollment_status(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    enrollment_service::get_enrollment_status(db.get_ref(), &session, path.into_inner()).await
}
