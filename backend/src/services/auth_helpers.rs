use actix_session::Session;
use actix_web::HttpResponse;
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

// true if user ONLY has student role — staff with student role still get staff access
pub fn is_student_only(role_ids: &[i32]) -> bool {
    role_ids.contains(&4) && !role_ids.iter().any(|r| [1, 2, 3].contains(r))
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

    let is_admin = role_names
        .iter()
        .any(|role| role == "LMS Admin" || role == "Organisation Admin");

    if is_admin {
        Ok(())
    } else {
        Err(HttpResponse::Forbidden().body("Admin access required"))
    }
}