use actix_session::Session;
use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::DatabaseConnection;

use crate::models::assignment::{CreateAssignment, UpdateAssignment};
use crate::services::assignment_service;

#[get("/assignment")]
pub async fn get_assignment(db: web::Data<DatabaseConnection>) -> impl Responder {
    assignment_service::list_assignments(db.get_ref()).await
}

#[get("/assignment/{course_id}")]
pub async fn get_assignment_by_course_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    assignment_service::list_assignments_by_course(db.get_ref(), path.into_inner()).await
}

#[put("/assignment/{assignment_id}")]
pub async fn update_assignment(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateAssignment>,
) -> impl Responder {
    assignment_service::update_assignment(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    ).await
}

#[post("/assignment")]
pub async fn create_assignment(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateAssignment>,
) -> impl Responder {
    assignment_service::create_assignment(db.get_ref(), &session, body.into_inner()).await
}

#[delete("/assignment/{assignment_id}")]
pub async fn delete_assignment(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    assignment_service::delete_assignment(db.get_ref(), &session, path.into_inner()).await
}
