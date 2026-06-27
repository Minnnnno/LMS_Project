use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use rust_decimal::Decimal;

use crate::entity::{assignment_prerequisites, assignments, courses, modules};
use crate::models::assignment::{AssignmentResponse, CreateAssignment, UpdateAssignment};
use crate::services::course_service::can_manage_course;
use crate::services::prerequisite_service;

async fn require_can_manage_course(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> Result<(), HttpResponse> {
    let course = courses::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    match can_manage_course(db, session, &course).await {
        Ok(true) => Ok(()),
        Ok(false) => {
            Err(HttpResponse::Forbidden().body("You cannot manage assignments for this course"))
        }
        Err(response) => Err(response),
    }
}

async fn require_module_in_course(
    db: &DatabaseConnection,
    module_id: i32,
    course_id: i32,
) -> Result<modules::Model, HttpResponse> {
    let module = modules::Entity::find_by_id(module_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding module: {}", err))
        })?
        .ok_or_else(|| HttpResponse::BadRequest().body("Selected module does not exist"))?;

    if module.course_id != course_id {
        return Err(
            HttpResponse::BadRequest().body("Selected module does not belong to this course")
        );
    }

    Ok(module)
}

fn validate_passing_mark(value: Decimal) -> Result<Decimal, HttpResponse> {
    if value < Decimal::ZERO || value > Decimal::new(100, 0) {
        return Err(HttpResponse::BadRequest().body("Passing mark must be between 0 and 100"));
    }

    Ok(value)
}

fn to_assignment_response(
    assignment: assignments::Model,
    prerequisite_module_ids: Vec<i32>,
) -> AssignmentResponse {
    AssignmentResponse {
        assignment_id: assignment.assignment_id,
        course_id: assignment.course_id,
        module_id: assignment.module_id,
        title: assignment.title,
        description: assignment.description,
        due_date: assignment.due_date,
        max_score: assignment.max_score,
        passing_mark: assignment.passing_mark,
        assignment_brief_url: assignment.assignment_brief_url,
        expected_file_type: assignment.expected_file_type,
        allow_text_submission: assignment.allow_text_submission,
        allow_file_submission: assignment.allow_file_submission,
        max_file_size_mb: assignment.max_file_size_mb,
        submission_instructions: assignment.submission_instructions,
        prerequisite_module_ids,
    }
}

async fn attach_assignment_prerequisites(
    db: &DatabaseConnection,
    assignment_rows: Vec<assignments::Model>,
) -> Result<Vec<AssignmentResponse>, HttpResponse> {
    let assignment_ids: Vec<i32> = assignment_rows
        .iter()
        .map(|assignment| assignment.assignment_id)
        .collect();

    let prerequisite_rows = if assignment_ids.is_empty() {
        Vec::new()
    } else {
        assignment_prerequisites::Entity::find()
            .filter(assignment_prerequisites::Column::AssignmentId.is_in(assignment_ids))
            .order_by_asc(assignment_prerequisites::Column::PrerequisiteId)
            .all(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!(
                    "Database error finding assignment prerequisites: {}",
                    err
                ))
            })?
    };

    let mut prerequisites_by_assignment = std::collections::HashMap::<i32, Vec<i32>>::new();
    for row in prerequisite_rows {
        prerequisites_by_assignment
            .entry(row.assignment_id)
            .or_default()
            .push(row.required_module_id);
    }

    Ok(assignment_rows
        .into_iter()
        .map(|assignment| {
            let prerequisite_module_ids = prerequisites_by_assignment
                .remove(&assignment.assignment_id)
                .unwrap_or_default();
            to_assignment_response(assignment, prerequisite_module_ids)
        })
        .collect())
}

pub async fn list_assignments(db: &DatabaseConnection) -> HttpResponse {
    match assignments::Entity::find()
        .order_by_asc(assignments::Column::CourseId)
        .order_by_asc(assignments::Column::ModuleId)
        .order_by_asc(assignments::Column::DueDate)
        .all(db)
        .await
    {
        Ok(assignments) => match attach_assignment_prerequisites(db, assignments).await {
            Ok(payload) => HttpResponse::Ok().json(payload),
            Err(response) => response,
        },
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_assignments_by_course(db: &DatabaseConnection, course_id: i32) -> HttpResponse {
    match assignments::Entity::find()
        .filter(assignments::Column::CourseId.eq(course_id))
        .order_by_asc(assignments::Column::ModuleId)
        .order_by_asc(assignments::Column::DueDate)
        .all(db)
        .await
    {
        Ok(assignments) => match attach_assignment_prerequisites(db, assignments).await {
            Ok(payload) => HttpResponse::Ok().json(payload),
            Err(response) => response,
        },
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn update_assignment(
    db: &DatabaseConnection,
    session: &Session,
    assignment_id: i32,
    data: UpdateAssignment,
) -> HttpResponse {
    match assignments::Entity::find_by_id(assignment_id).one(db).await {
        Ok(Some(assignment)) => {
            if let Err(response) =
                require_can_manage_course(db, session, assignment.course_id).await
            {
                return response;
            }

            let target_course_id = data.course_id.unwrap_or(assignment.course_id);
            let target_module_id = data.module_id.unwrap_or(assignment.module_id);

            if target_course_id != assignment.course_id {
                if let Err(response) =
                    require_can_manage_course(db, session, target_course_id).await
                {
                    return response;
                }
            }

            if let Err(response) =
                require_module_in_course(db, target_module_id, target_course_id).await
            {
                return response;
            }

            let mut active: assignments::ActiveModel = assignment.into();

            if let Some(course_id) = data.course_id {
                active.course_id = Set(course_id);
            }

            if let Some(module_id) = data.module_id {
                active.module_id = Set(module_id);
            }

            if let Some(title) = data.title {
                active.title = Set(title);
            }

            if let Some(description) = data.description {
                active.description = Set(Some(description));
            }

            if let Some(due_date) = data.due_date {
                active.due_date = Set(Some(due_date));
            }

            if let Some(max_score) = data.max_score {
                active.max_score = Set(Some(max_score));
            }

            if let Some(passing_mark) = data.passing_mark {
                let passing_mark = match validate_passing_mark(passing_mark) {
                    Ok(value) => value,
                    Err(response) => return response,
                };
                active.passing_mark = Set(passing_mark);
            }

            if let Some(assignment_brief_url) = data.assignment_brief_url {
                active.assignment_brief_url = Set(assignment_brief_url);
            }

            if let Some(expected_file_type) = data.expected_file_type {
                active.expected_file_type = Set(expected_file_type);
            }

            if let Some(allow_text_submission) = data.allow_text_submission {
                active.allow_text_submission = Set(Some(allow_text_submission));
            }

            if let Some(allow_file_submission) = data.allow_file_submission {
                active.allow_file_submission = Set(Some(allow_file_submission));
            }

            if let Some(max_file_size_mb) = data.max_file_size_mb {
                active.max_file_size_mb = Set(max_file_size_mb);
            }

            if let Some(submission_instructions) = data.submission_instructions {
                active.submission_instructions = Set(submission_instructions);
            }
            match active.update(db).await {
                Ok(saved) => {
                    if let Some(prerequisite_module_ids) = data.prerequisite_module_ids {
                        if let Err(response) =
                            prerequisite_service::replace_assignment_prerequisites(
                                db,
                                target_course_id,
                                saved.assignment_id,
                                prerequisite_module_ids,
                            )
                            .await
                        {
                            return response;
                        }
                    }

                    HttpResponse::Ok()
                        .body(format!("Assignment with id {} updated!", assignment_id))
                }
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Update error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Assignment not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn create_assignment(
    db: &DatabaseConnection,
    session: &Session,
    data: CreateAssignment,
) -> HttpResponse {
    if let Err(response) = require_can_manage_course(db, session, data.course_id).await {
        return response;
    }
    if let Err(response) = require_module_in_course(db, data.module_id, data.course_id).await {
        return response;
    }

    let passing_mark = match validate_passing_mark(data.passing_mark.unwrap_or(Decimal::new(50, 0)))
    {
        Ok(value) => value,
        Err(response) => return response,
    };

    let assignment = assignments::ActiveModel {
        course_id: Set(data.course_id),
        module_id: Set(data.module_id),
        title: Set(data.title),
        description: Set(Some(data.description)),
        due_date: Set(Some(data.due_date)),
        max_score: Set(Some(data.max_score)),
        passing_mark: Set(passing_mark),
        assignment_brief_url: Set(data.assignment_brief_url),
        expected_file_type: Set(data.expected_file_type),
        allow_text_submission: Set(Some(data.allow_text_submission.unwrap_or(true))),
        allow_file_submission: Set(Some(data.allow_file_submission.unwrap_or(true))),
        max_file_size_mb: Set(data.max_file_size_mb),
        submission_instructions: Set(data.submission_instructions),
        ..Default::default()
    };

    match assignment.insert(db).await {
        Ok(saved) => {
            if let Err(response) = prerequisite_service::replace_assignment_prerequisites(
                db,
                saved.course_id,
                saved.assignment_id,
                data.prerequisite_module_ids,
            )
            .await
            {
                return response;
            }

            HttpResponse::Ok().body("New assignment created successfully!")
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn delete_assignment(
    db: &DatabaseConnection,
    session: &Session,
    assignment_id: i32,
) -> HttpResponse {
    match assignments::Entity::find_by_id(assignment_id).one(db).await {
        Ok(Some(assignment)) => {
            if let Err(response) =
                require_can_manage_course(db, session, assignment.course_id).await
            {
                return response;
            }

            let active_model: assignments::ActiveModel = assignment.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Assignment deleted!"),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Assignment not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
