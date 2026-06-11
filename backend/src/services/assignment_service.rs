use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::{assignments, courses};
use crate::models::assignment::{CreateAssignment, UpdateAssignment};
use crate::services::course_service::can_manage_course;

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
        Ok(false) => Err(HttpResponse::Forbidden().body("You cannot manage assignments for this course")),
        Err(response) => Err(response),
    }
}

pub async fn list_assignments(db: &DatabaseConnection) -> HttpResponse {
    match assignments::Entity::find().all(db).await {
        Ok(assignments) => HttpResponse::Ok().json(assignments),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_assignments_by_course(db: &DatabaseConnection, course_id: i32) -> HttpResponse {
    match assignments::Entity::find()
        .filter(assignments::Column::CourseId.eq(course_id))
        .all(db)
        .await
    {
        Ok(assignments) => HttpResponse::Ok().json(assignments),
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
            if let Err(response) = require_can_manage_course(db, session, assignment.course_id).await {
                return response;
            }

            if let Some(course_id) = data.course_id {
                if course_id != assignment.course_id {
                    if let Err(response) = require_can_manage_course(db, session, course_id).await {
                        return response;
                    }
                }
            }

            let mut active: assignments::ActiveModel = assignment.into();

            if let Some(course_id) = data.course_id {
                active.course_id = Set(course_id);
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
                Ok(_) => HttpResponse::Ok().body(format!("Assignment with id {} updated!", assignment_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
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

    let assignment = assignments::ActiveModel {
        course_id: Set(data.course_id),
        title: Set(data.title),
        description: Set(Some(data.description)),
        due_date: Set(Some(data.due_date)),
        max_score: Set(Some(data.max_score)),
        assignment_brief_url: Set(data.assignment_brief_url),
        expected_file_type: Set(data.expected_file_type),
        allow_text_submission: Set(Some(data.allow_text_submission.unwrap_or(true))),
        allow_file_submission: Set(Some(data.allow_file_submission.unwrap_or(true))),
        max_file_size_mb: Set(data.max_file_size_mb),
        submission_instructions: Set(data.submission_instructions),
        ..Default::default()
    };

    match assignment.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("New assignment created successfully!"),
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
            if let Err(response) = require_can_manage_course(db, session, assignment.course_id).await {
                return response;
            }

            let active_model: assignments::ActiveModel = assignment.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Assignment deleted!"),
                Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Assignment not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
