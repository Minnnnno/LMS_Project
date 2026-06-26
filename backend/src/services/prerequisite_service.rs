use std::collections::HashSet;

use actix_web::HttpResponse;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set,
};

use crate::entity::{
    assignment_prerequisites, module_prerequisites, module_progress, modules, quiz_prerequisites,
};
use crate::services::quiz_helper::{QuizResult, QuizServiceError};

fn unique_module_ids(module_ids: Vec<i32>) -> Vec<i32> {
    let mut seen = HashSet::new();
    module_ids
        .into_iter()
        .filter(|module_id| seen.insert(*module_id))
        .collect()
}

pub async fn get_module_prerequisite_ids(
    db: &DatabaseConnection,
    module_id: i32,
) -> Result<Vec<i32>, HttpResponse> {
    module_prerequisites::Entity::find()
        .filter(module_prerequisites::Column::ModuleId.eq(module_id))
        .order_by_asc(module_prerequisites::Column::PrerequisiteId)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error finding module prerequisites: {}",
                err
            ))
        })
        .map(|rows| rows.into_iter().map(|row| row.required_module_id).collect())
}

pub async fn get_quiz_prerequisite_ids_for_service(
    db: &impl ConnectionTrait,
    quiz_id: i32,
) -> QuizResult<Vec<i32>> {
    quiz_prerequisites::Entity::find()
        .filter(quiz_prerequisites::Column::QuizId.eq(quiz_id))
        .order_by_asc(quiz_prerequisites::Column::PrerequisiteId)
        .all(db)
        .await
        .map_err(|err| {
            QuizServiceError::Internal(format!(
                "Database error finding quiz prerequisites: {}",
                err
            ))
        })
        .map(|rows| rows.into_iter().map(|row| row.required_module_id).collect())
}

pub async fn get_assignment_prerequisite_ids(
    db: &DatabaseConnection,
    assignment_id: i32,
) -> Result<Vec<i32>, HttpResponse> {
    assignment_prerequisites::Entity::find()
        .filter(assignment_prerequisites::Column::AssignmentId.eq(assignment_id))
        .order_by_asc(assignment_prerequisites::Column::PrerequisiteId)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error finding assignment prerequisites: {}",
                err
            ))
        })
        .map(|rows| rows.into_iter().map(|row| row.required_module_id).collect())
}

async fn validate_required_modules(
    db: &impl ConnectionTrait,
    course_id: i32,
    target_module_id: Option<i32>,
    required_module_ids: &[i32],
) -> QuizResult<()> {
    if required_module_ids.is_empty() {
        return Ok(());
    }

    if let Some(module_id) = target_module_id {
        if required_module_ids.contains(&module_id) {
            return Err(QuizServiceError::BadRequest(
                "A module cannot require itself".to_string(),
            ));
        }
    }

    let required_modules = modules::Entity::find()
        .filter(modules::Column::ModuleId.is_in(required_module_ids.to_vec()))
        .all(db)
        .await
        .map_err(|err| {
            QuizServiceError::Internal(format!(
                "Database error finding prerequisite modules: {}",
                err
            ))
        })?;

    if required_modules.len() != required_module_ids.len() {
        return Err(QuizServiceError::BadRequest(
            "One or more prerequisite modules do not exist".to_string(),
        ));
    }

    if required_modules
        .iter()
        .any(|module| module.course_id != course_id)
    {
        return Err(QuizServiceError::BadRequest(
            "Prerequisite modules must belong to the same course".to_string(),
        ));
    }

    Ok(())
}

pub async fn replace_module_prerequisites(
    db: &DatabaseConnection,
    course_id: i32,
    module_id: i32,
    required_module_ids: Vec<i32>,
) -> Result<(), HttpResponse> {
    let required_module_ids = unique_module_ids(required_module_ids);
    validate_required_modules(db, course_id, Some(module_id), &required_module_ids)
        .await
        .map_err(QuizServiceError::into_response)?;

    module_prerequisites::Entity::delete_many()
        .filter(module_prerequisites::Column::ModuleId.eq(module_id))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error clearing module prerequisites: {}",
                err
            ))
        })?;

    for required_module_id in required_module_ids {
        module_prerequisites::ActiveModel {
            module_id: Set(module_id),
            required_module_id: Set(required_module_id),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error saving module prerequisite: {}",
                err
            ))
        })?;
    }

    Ok(())
}

pub async fn replace_quiz_prerequisites_for_service(
    db: &impl ConnectionTrait,
    course_id: i32,
    quiz_id: i32,
    required_module_ids: Vec<i32>,
) -> QuizResult<()> {
    let required_module_ids = unique_module_ids(required_module_ids);
    validate_required_modules(db, course_id, None, &required_module_ids).await?;

    quiz_prerequisites::Entity::delete_many()
        .filter(quiz_prerequisites::Column::QuizId.eq(quiz_id))
        .exec(db)
        .await
        .map_err(|err| {
            QuizServiceError::Internal(format!(
                "Database error clearing quiz prerequisites: {}",
                err
            ))
        })?;

    for required_module_id in required_module_ids {
        quiz_prerequisites::ActiveModel {
            quiz_id: Set(quiz_id),
            required_module_id: Set(required_module_id),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| {
            QuizServiceError::Internal(format!("Database error saving quiz prerequisite: {}", err))
        })?;
    }

    Ok(())
}

pub async fn replace_assignment_prerequisites(
    db: &DatabaseConnection,
    course_id: i32,
    assignment_id: i32,
    required_module_ids: Vec<i32>,
) -> Result<(), HttpResponse> {
    let required_module_ids = unique_module_ids(required_module_ids);
    validate_required_modules(db, course_id, None, &required_module_ids)
        .await
        .map_err(QuizServiceError::into_response)?;

    assignment_prerequisites::Entity::delete_many()
        .filter(assignment_prerequisites::Column::AssignmentId.eq(assignment_id))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error clearing assignment prerequisites: {}",
                err
            ))
        })?;

    for required_module_id in required_module_ids {
        assignment_prerequisites::ActiveModel {
            assignment_id: Set(assignment_id),
            required_module_id: Set(required_module_id),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error saving assignment prerequisite: {}",
                err
            ))
        })?;
    }

    Ok(())
}

pub async fn get_first_incomplete_required_module(
    db: &impl ConnectionTrait,
    user_id: i32,
    required_module_ids: Vec<i32>,
) -> Result<Option<modules::Model>, HttpResponse> {
    get_first_incomplete_required_module_for_service(db, user_id, required_module_ids)
        .await
        .map_err(QuizServiceError::into_response)
}

pub async fn get_first_incomplete_required_module_for_service(
    db: &impl ConnectionTrait,
    user_id: i32,
    required_module_ids: Vec<i32>,
) -> QuizResult<Option<modules::Model>> {
    let required_module_ids = unique_module_ids(required_module_ids);

    if required_module_ids.is_empty() {
        return Ok(None);
    }

    let completed_ids: HashSet<i32> = module_progress::Entity::find()
        .filter(module_progress::Column::UserId.eq(user_id))
        .filter(module_progress::Column::ModuleId.is_in(required_module_ids.clone()))
        .filter(module_progress::Column::CompletedAt.is_not_null())
        .all(db)
        .await
        .map_err(|err| {
            QuizServiceError::Internal(format!("Database error finding module progress: {}", err))
        })?
        .into_iter()
        .map(|progress| progress.module_id)
        .collect();

    let incomplete_ids: Vec<i32> = required_module_ids
        .into_iter()
        .filter(|module_id| !completed_ids.contains(module_id))
        .collect();

    if incomplete_ids.is_empty() {
        return Ok(None);
    }

    modules::Entity::find()
        .filter(modules::Column::ModuleId.is_in(incomplete_ids))
        .order_by_asc(modules::Column::Position)
        .one(db)
        .await
        .map_err(|err| {
            QuizServiceError::Internal(format!(
                "Database error finding prerequisite module: {}",
                err
            ))
        })
}
