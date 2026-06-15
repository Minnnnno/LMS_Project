use actix_session::Session;
use actix_web::HttpResponse;
use chrono::Utc;
use std::collections::HashSet;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};

use crate::entity::{courses, module_progress, modules};
use crate::models::module_progress::{CourseModuleProgress, CourseProgress, ModuleProgress};
use crate::services::auth_helpers::{get_user_id, is_enrolled};
use crate::services::course_service::can_manage_course;

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

pub async fn get_course_progress(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> Result<CourseProgress, HttpResponse> {
    let user_id = get_user_id(session)?;

    let course = courses::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    let enrolled = is_enrolled(db, user_id, course_id).await?;
    let can_manage = can_manage_course(db, session, &course).await?;

    if !enrolled && !can_manage {
        return Err(HttpResponse::Forbidden().body("You must be enrolled to view course progress"));
    }

    let course_modules = modules::Entity::find()
        .filter(modules::Column::CourseId.eq(course_id))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding modules: {}", err))
        })?;

    let total_modules = course_modules.len() as u64;

    if total_modules == 0 {
        return Ok(CourseProgress {
            completed_modules: 0,
            total_modules,
            progress_percent: 0,
        });
    }

    let module_ids: Vec<i32> = course_modules
        .into_iter()
        .map(|module| module.module_id)
        .collect();

    let completed_modules = module_progress::Entity::find()
        .filter(module_progress::Column::UserId.eq(user_id))
        .filter(module_progress::Column::ModuleId.is_in(module_ids))
        .filter(module_progress::Column::CompletedAt.is_not_null())
        .count(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error counting module progress: {}", err))
        })?;

    let progress_percent = ((completed_modules * 100) / total_modules)
        .min(100)
        .try_into()
        .unwrap_or(100);

    Ok(CourseProgress {
        completed_modules,
        total_modules,
        progress_percent,
    })
}

pub async fn get_course_module_progress(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> Result<Vec<CourseModuleProgress>, HttpResponse> {
    let user_id = get_user_id(session)?;

    let course = courses::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    let enrolled = is_enrolled(db, user_id, course_id).await?;
    let can_manage = can_manage_course(db, session, &course).await?;

    if !enrolled && !can_manage {
        return Err(HttpResponse::Forbidden().body("You must be enrolled to view module progress"));
    }

    let course_modules = modules::Entity::find()
        .filter(modules::Column::CourseId.eq(course_id))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding modules: {}", err))
        })?;

    let module_ids: Vec<i32> = course_modules
        .iter()
        .map(|module| module.module_id)
        .collect();

    if module_ids.is_empty() {
        return Ok(Vec::new());
    }

    let completed_ids: HashSet<i32> = module_progress::Entity::find()
        .filter(module_progress::Column::UserId.eq(user_id))
        .filter(module_progress::Column::ModuleId.is_in(module_ids))
        .filter(module_progress::Column::CompletedAt.is_not_null())
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding module progress: {}", err))
        })?
        .into_iter()
        .map(|progress| progress.module_id)
        .collect();

    Ok(course_modules
        .into_iter()
        .map(|module| {
            let opened = completed_ids.contains(&module.module_id);

            CourseModuleProgress {
                module_id: module.module_id,
                opened,
                progress_percent: if opened { 100 } else { 0 },
            }
        })
        .collect())
}
