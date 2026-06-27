use actix_session::Session;
use actix_web::{Responder, get, put, web};
use sea_orm::DatabaseConnection;

use crate::models::student::{ChangePasswordForm, UpdateOwnProfileForm};
use crate::services::student_service;

#[get("/student/profile")]
pub async fn get_own_profile(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    student_service::get_own_profile(db.get_ref(), &session).await
}

#[put("/student/profile")]
pub async fn update_own_profile(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<UpdateOwnProfileForm>,
) -> impl Responder {
    student_service::update_own_profile(db.get_ref(), &session, body.into_inner()).await
}

#[put("/student/password")]
pub async fn change_password(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<ChangePasswordForm>,
) -> impl Responder {
    student_service::change_password(db.get_ref(), &session, body.into_inner()).await
}
