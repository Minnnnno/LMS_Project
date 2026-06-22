use actix_session::Session;
use actix_web::HttpResponse;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::{courses, module_progress, modules};
use crate::models::module_progress::ModuleProgress;
use crate::services::auth_helpers::{get_user_id, is_enrolled};
use crate::services::course_service::can_manage_course;
use crate::services::prerequisite_service;

async fn get_module_course(
    db: &DatabaseConnection,
    module_id: i32,
) -> Result<(modules::Model, courses::Model), HttpResponse> {
    let module = modules::Entity::find_by_id(module_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding module: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Module not found"))?;

    let course = courses::Entity::find_by_id(module.course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    Ok((module, course))
}

pub async fn get_first_incomplete_previous_module(
    db: &DatabaseConnection,
    session: &Session,
    module_id: i32,
) -> Result<Option<modules::Model>, HttpResponse> {
    let user_id = get_user_id(session)?;
    let (_module, course) = get_module_course(db, module_id).await?;

    if can_manage_course(db, session, &course).await? {
        return Ok(None);
    }

    if !is_enrolled(db, user_id, course.course_id).await? {
        return Err(HttpResponse::Forbidden().body("You must be enrolled to view module content"));
    }

    let prerequisite_ids = prerequisite_service::get_module_prerequisite_ids(db, module_id).await?;

    prerequisite_service::get_first_incomplete_required_module(db, user_id, prerequisite_ids).await
}

pub async fn mark_module_completed(
    db: &DatabaseConnection,
    session: &Session,
    module_id: i32,
) -> Result<ModuleProgress, HttpResponse> {
    let user_id = get_user_id(session)?;
    let (module, course) = get_module_course(db, module_id).await?;

    if !is_enrolled(db, user_id, course.course_id).await? {
        return Ok(ModuleProgress {
            opened: false,
            progress_percent: 0,
        });
    }

    if let Some(prerequisite) = get_first_incomplete_previous_module(db, session, module_id).await?
    {
        return Err(HttpResponse::Forbidden().body(format!(
            "Complete {} before opening this module",
            prerequisite.title
        )));
    }

    let completed_at = Utc::now();
    let existing = module_progress::Entity::find()
        .filter(module_progress::Column::UserId.eq(user_id))
        .filter(module_progress::Column::ModuleId.eq(module.module_id))
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding module progress: {}", err))
        })?;

    if let Some(progress) = existing {
        let mut active: module_progress::ActiveModel = progress.into();
        active.completed_at = Set(Some(completed_at));

        active.update(db).await.map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error updating module progress: {}", err))
        })?;
    } else {
        let progress = module_progress::ActiveModel {
            user_id: Set(user_id),
            module_id: Set(module.module_id),
            completed_at: Set(Some(completed_at)),
        };

        progress.insert(db).await.map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error saving module progress: {}", err))
        })?;
    }

    Ok(ModuleProgress {
        opened: true,
        progress_percent: 100,
    })
}

pub async fn get_module_progress(
    db: &DatabaseConnection,
    session: &Session,
    module_id: i32,
) -> Result<ModuleProgress, HttpResponse> {
    let user_id = get_user_id(session)?;
    let (module, course) = get_module_course(db, module_id).await?;
    let enrolled = is_enrolled(db, user_id, course.course_id).await?;
    let can_manage = can_manage_course(db, session, &course).await?;

    if !enrolled && !can_manage {
        return Err(HttpResponse::Forbidden().body("You must be enrolled to view module progress"));
    }

    let opened = module_progress::Entity::find()
        .filter(module_progress::Column::UserId.eq(user_id))
        .filter(module_progress::Column::ModuleId.eq(module.module_id))
        .filter(module_progress::Column::CompletedAt.is_not_null())
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding module progress: {}", err))
        })?
        .is_some();

    Ok(ModuleProgress {
        opened,
        progress_percent: if opened { 100 } else { 0 },
    })
}
