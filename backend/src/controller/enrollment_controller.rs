use actix_session::Session;
use actix_web::{post, web, Responder};
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
