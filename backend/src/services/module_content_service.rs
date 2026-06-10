use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::entity::{courses, modules, users};
use crate::services::auth_helpers::{get_user_id, is_enrolled};
use crate::services::course_service::has_role;

pub async fn get_course_for_module(
    db: &DatabaseConnection,
    module_id: i32,
) -> Result<courses::Model, HttpResponse> {
    let module = modules::Entity::find_by_id(module_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding module: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Module not found"))?;

    courses::Entity::find_by_id(module.course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))
}

pub async fn can_manage_module_content(
    db: &DatabaseConnection,
    session: &Session,
    module_id: i32,
) -> Result<bool, HttpResponse> {
    let user_id = get_user_id(session)?;

    if has_role(session, "LMS Admin") {
        return Ok(true);
    }

    if !has_role(session, "Organisation Admin") {
        return Ok(false);
    }

    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding user: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("User not found"))?;

    let course = get_course_for_module(db, module_id).await?;

    Ok(user.org_id.is_some() && user.org_id == course.org_id)
}

pub async fn can_view_module_content(
    db: &DatabaseConnection,
    session: &Session,
    module_id: i32,
) -> Result<bool, HttpResponse> {
    if can_manage_module_content(db, session, module_id).await? {
        return Ok(true);
    }

    let user_id = get_user_id(session)?;
    let course = get_course_for_module(db, module_id).await?;

    is_enrolled(db, user_id, course.course_id).await
}
