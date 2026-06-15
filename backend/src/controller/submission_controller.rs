use actix_session::Session;
use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::DatabaseConnection;

use crate::models::submission::{CreateSubmission, GradeSubmission};
use crate::services::submission_service;

#[post("/assignments/{assignment_id}/submissions")]
pub async fn create_submission(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<CreateSubmission>,
) -> impl Responder {
    submission_service::create_submission(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    )
    .await
}

#[get("/assignments/{assignment_id}/submissions/my")]
pub async fn list_my_submissions(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    submission_service::list_my_submissions(db.get_ref(), &session, path.into_inner()).await
}

#[get("/assignments/{assignment_id}/submissions")]
pub async fn list_assignment_submissions(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    submission_service::list_assignment_submissions(db.get_ref(), &session, path.into_inner()).await
}

#[put("/submissions/{submission_id}/grade")]
pub async fn grade_submission(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<GradeSubmission>,
) -> impl Responder {
    submission_service::grade_submission(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    )
    .await
}

#[delete("/submissions/{submission_id}/grade")]
pub async fn clear_submission_grade(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    submission_service::clear_submission_grade(db.get_ref(), &session, path.into_inner()).await
}
