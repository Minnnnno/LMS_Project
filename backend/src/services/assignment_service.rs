use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::assignments;
use crate::models::assignment::{CreateAssignment, UpdateAssignment};

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
        Ok(assignments) if assignments.is_empty() => HttpResponse::NotFound().body("No assignments found"),
        Ok(assignments) => HttpResponse::Ok().json(assignments),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn update_assignment(
    db: &DatabaseConnection,
    assignment_id: i32,
    data: UpdateAssignment,
) -> HttpResponse {
    match assignments::Entity::find_by_id(assignment_id).one(db).await {
        Ok(Some(assignment)) => {
            let mut active: assignments::ActiveModel = assignment.into();

            if let Some(course_id) = data.course_id {
                active.course_id = Set(course_id);
            }
            if let Some(title) = data.title {
                active.title = Set(title);
            }
            if let Some(description) = data.description {
                active.description = Set(description);
            }
            if let Some(due_date) = data.due_date {
                active.due_date = Set(due_date);
            }
            if let Some(max_score) = data.max_score {
                active.max_score = Set(max_score);
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

pub async fn create_assignment(db: &DatabaseConnection, data: CreateAssignment) -> HttpResponse {
    let assignment = assignments::ActiveModel {
        course_id: Set(data.course_id),
        title: Set(data.title),
        description: Set(data.description),
        due_date: Set(data.due_date),
        max_score: Set(data.max_score),
        ..Default::default()
    };

    match assignment.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("New assignment created successfully!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn delete_assignment(db: &DatabaseConnection, assignment_id: i32) -> HttpResponse {
    match assignments::Entity::find_by_id(assignment_id).one(db).await {
        Ok(Some(assignment)) => {
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
