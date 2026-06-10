use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::quiz::{self, Entity as QuizEntity};
use crate::models::quiz::{CreateQuiz, UpdateQuiz};

pub async fn list_quizzes(db: &DatabaseConnection) -> HttpResponse {
    match QuizEntity::find().all(db).await {
        Ok(quizzes) if quizzes.is_empty() => HttpResponse::NotFound().body("No quizzes found"),
        Ok(quizzes) => HttpResponse::Ok().json(quizzes),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_quizzes_by_course(db: &DatabaseConnection, course_id: i32) -> HttpResponse {
    match QuizEntity::find()
        .filter(quiz::Column::CourseId.eq(course_id))
        .all(db)
        .await
    {
        Ok(quizzes) if quizzes.is_empty() => HttpResponse::NotFound().body("No quizzes found"),
        Ok(quizzes) => HttpResponse::Ok().json(quizzes),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn create_quiz(db: &DatabaseConnection, data: CreateQuiz) -> HttpResponse {
    let new_quiz = quiz::ActiveModel {
        course_id: Set(data.course_id),
        title: Set(data.title),
        description: Set(data.description),
        max_attempts: Set(data.max_attempts),
        time_limit: Set(data.time_limit),
        starts_at: Set(data.starts_at),
        ..Default::default()
    };

    match new_quiz.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("New quiz created successfully!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn update_quiz(db: &DatabaseConnection, quiz_id: i32, data: UpdateQuiz) -> HttpResponse {
    match QuizEntity::find_by_id(quiz_id).one(db).await {
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

            match active.update(db).await {
                Ok(_) => HttpResponse::Ok().body(format!("Quiz with id {} updated!", quiz_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Quiz not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn delete_quiz(db: &DatabaseConnection, quiz_id: i32) -> HttpResponse {
    match QuizEntity::find_by_id(quiz_id).one(db).await {
        Ok(Some(target_quiz)) => {
            let active_model: quiz::ActiveModel = target_quiz.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Quiz deleted!"),
                Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Quiz not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
