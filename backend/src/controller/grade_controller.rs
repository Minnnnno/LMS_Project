use actix_session::Session;
use actix_web::{Responder, get, web};
use sea_orm::DatabaseConnection;

use crate::services::grade_service;

#[get("/courses/{course_id}/grades")]
pub async fn get_my_course_grades(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    grade_service::get_my_course_grades(db.get_ref(), &session, path.into_inner()).await
}
