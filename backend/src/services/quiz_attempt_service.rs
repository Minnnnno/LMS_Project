use actix_web::HttpResponse;
use chrono::{Local, NaiveDateTime};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::quiz_attempts::{
    ActiveModel as QuizAttemptActiveModel, Column as QuizAttemptColumn,
    Entity as QuizAttemptEntity,
};
use crate::models::quiz_attempts::{CreateAttempt, MarkAttempt};

pub async fn list_attempts(db: &DatabaseConnection) -> HttpResponse {
    match QuizAttemptEntity::find().all(db).await {
        Ok(attempts) if attempts.is_empty() => HttpResponse::NotFound().body("No quiz attempts found"),
        Ok(attempts) => HttpResponse::Ok().json(attempts),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_attempts_by_quiz(db: &DatabaseConnection, quiz_id: i32) -> HttpResponse {
    match QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::QuizId.eq(quiz_id))
        .all(db)
        .await
    {
        Ok(attempts) if attempts.is_empty() => HttpResponse::NotFound().body("No attempts found"),
        Ok(attempts) => HttpResponse::Ok().json(attempts),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn create_attempt(db: &DatabaseConnection, data: CreateAttempt) -> HttpResponse {
    let attempt = QuizAttemptActiveModel {
        quiz_id: Set(data.quiz_id),
        user_id: Set(data.user_id),
        ..Default::default()
    };

    match attempt.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("New attempt created successfully!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn submit_attempt(db: &DatabaseConnection, attempt_id: i32) -> HttpResponse {
    match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => {
            let mut active: QuizAttemptActiveModel = attempt.into();
            let now: NaiveDateTime = Local::now().naive_local();
            active.submitted_at = Set(Some(now));

            match active.update(db).await {
                Ok(_) => HttpResponse::Ok().body(format!("Attempt {} marked as submitted", attempt_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn grade_attempt(
    db: &DatabaseConnection,
    attempt_id: i32,
    data: MarkAttempt,
) -> HttpResponse {
    match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => {
            let mut active: QuizAttemptActiveModel = attempt.into();

            if let Some(total_score) = data.total_score {
                active.total_score = Set(Some(total_score));
            }

            match active.update(db).await {
                Ok(_) => HttpResponse::Ok().body(format!("Score for attempt {} updated", attempt_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn delete_attempt(db: &DatabaseConnection, attempt_id: i32) -> HttpResponse {
    match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => {
            let active_model: QuizAttemptActiveModel = attempt.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Attempt deleted!"),
                Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
