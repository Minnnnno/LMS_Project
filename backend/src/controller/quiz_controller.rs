use actix_session::Session;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::entity::quiz::Entity as QuizEntity;
use crate::entity::{quiz, roles, user_roles};
use crate::models::quiz::{CreateQuiz, UpdateQuiz};

// SELECT * FROM quiz
#[get("/quiz")]
pub async fn get_quiz(
    db: web::Data<DatabaseConnection>
) -> impl Responder {
    let result = QuizEntity::find()
        .all(db.get_ref())
        .await;

    match result {
        Ok(quizzes) => {
            if quizzes.is_empty() {
                HttpResponse::NotFound().body("No quizzes found")
            } else {
                HttpResponse::Ok().json(quizzes)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

// SELECT * FROM quiz WHERE course_id =
#[get("/quiz/{course_id}")]
pub async fn get_quiz_by_course_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let course_id = path.into_inner();
    let result = QuizEntity::find()
        .filter(quiz::Column::CourseId.eq(course_id))
        .all(db.get_ref())
        .await;

    match result {
        Ok(quizzes) => {
            if quizzes.is_empty() {
                HttpResponse::NotFound().body("No quizzes found")
            } else {
                HttpResponse::Ok().json(quizzes)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[post("/quiz")]
pub async fn create_quiz(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateQuiz>
) -> impl Responder {
    let data = body.into_inner();
    let new_quiz = quiz::ActiveModel {
        course_id: Set(data.course_id),
        title: Set(data.title),
        description: Set(data.description),
        max_attempts: Set(data.max_attempts),
        time_limit: Set(data.time_limit),
        starts_at: Set(data.starts_at),
        ..Default::default()
    };

    match new_quiz.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("New quiz created successfully!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err)),
    }
}

#[put("/quiz/{quiz_id}")]
pub async fn update_quiz(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<UpdateQuiz>
) -> impl Responder {
    let quiz_id = path.into_inner();
    let data = body.into_inner();

    let existing = QuizEntity::find_by_id(quiz_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(updated_quiz)) => {
            let mut active: quiz::ActiveModel = updated_quiz.into();

            if let Some(course_id) = data.course_id {
                active.course_id = Set(course_id);
            }
            if let Some(title) = data.title {
                active.title = Set(title);
            }
            if let Some(description) = data.description {
                active.description = Set(Some(description));
            }
            if let Some(max_attempts) = data.max_attempts {
                active.max_attempts = Set(Some(max_attempts));
            }
            if let Some(time_limit) = data.time_limit {
                active.time_limit = Set(Some(time_limit));
            }
            if let Some(starts_at) = data.starts_at {
                active.starts_at = Set(Some(starts_at));
            }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok()
                    .body(format!("Quiz with id {} updated!", quiz_id)),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Quiz not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[delete("/quiz/{quiz_id}")]
pub async fn delete_quiz(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let quiz_id = path.into_inner();
    let existing = QuizEntity::find_by_id(quiz_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(updated_quiz)) => {
            let active_model: quiz::ActiveModel = updated_quiz.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Quiz deleted!"),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Quiz not found!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Delete error {}", err)),
    }
}