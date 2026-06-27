use actix_session::Session;
use actix_web::{HttpResponse, http::header};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::entity::enrollments;

pub fn get_user_id(session: &Session) -> Result<i32, HttpResponse> {
    match session.get::<i32>("user_id") {
        Ok(Some(id)) => Ok(id),
        Ok(None) => Err(HttpResponse::Unauthorized().body("You must be logged in")),
        Err(_) => Err(HttpResponse::InternalServerError().body("Session error")),
    }
}

pub fn get_role_ids(session: &Session) -> Vec<i32> {
    session
        .get::<Vec<i32>>("role_ids")
        .ok()
        .flatten()
        .unwrap_or_default()
}

pub fn redirect_to_login() -> HttpResponse {
    HttpResponse::Found()
        .insert_header((header::LOCATION, "/login"))
        .finish()
}

// true if user ONLY has student role — staff with student role still get staff access
pub fn has_staff_role(role_ids: &[i32]) -> bool {
    role_ids.iter().any(|role_id| [1, 2, 3].contains(role_id))
}

pub fn is_student_only(role_ids: &[i32]) -> bool {
    role_ids.contains(&4) && !has_staff_role(role_ids)
}

pub async fn is_enrolled(
    db: &DatabaseConnection,
    user_id: i32,
    course_id: i32,
) -> Result<bool, HttpResponse> {
    match enrollments::Entity::find()
        .filter(enrollments::Column::UserId.eq(user_id))
        .filter(enrollments::Column::CourseId.eq(course_id))
        .one(db)
        .await
    {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(err) => Err(HttpResponse::InternalServerError()
            .body(format!("Database error checking enrollment: {}", err))),
    }
}

pub fn require_admin(session: &Session) -> Result<(), HttpResponse> {
    let role_names = session
        .get::<Vec<String>>("role_names")
        .ok()
        .flatten()
        .unwrap_or_default();

    let is_admin = role_names.iter().any(|role| role == "LMS Admin");

    if is_admin {
        Ok(())
    } else {
        Err(HttpResponse::Forbidden().body("LMS Admin access required"))
    }
}

pub fn require_admin_page(session: &Session) -> Result<(), HttpResponse> {
    match session.get::<i32>("user_id") {
        Ok(Some(_)) => require_admin(session),
        Ok(None) => Err(redirect_to_login()),
        Err(_) => Err(HttpResponse::InternalServerError().body("Session error")),
    }
}
