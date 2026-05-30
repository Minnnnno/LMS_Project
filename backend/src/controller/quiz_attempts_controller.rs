use actix_session::Session;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use chrono::{Local, NaiveDateTime};

use crate::entity::quiz_attempts::{
    Entity as QuizAttemptEntity, 
    Column as QuizAttemptColumn,
    ActiveModel as QuizAttemptActiveModel,
};

use crate::entity::{roles, user_roles};
use crate::models::quiz_attempts::{CreateAttempt, MarkAttempt, SubmitAttempt};

//SELECT * FROM quiz
#[get("/quiz-attempts")]
pub async fn get_quiz_attempts(
    db: web::Data<DatabaseConnection>
) -> impl Responder {
    let result = QuizAttemptEntity::find()
    .all(db.get_ref())
    .await;
    match result {
        Ok(quiz_attempts) => {
            if quiz_attempts.is_empty(){
                HttpResponse::NotFound()
                .body("No quiz attempts found")
            }else{
                HttpResponse::Ok().json(quiz_attempts)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[get("/quiz-attempts/{quiz_id}")]
pub async fn get_attempts_by_quiz_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let quiz_id = path.into_inner();
    let result = QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::QuizId.eq(quiz_id))
        .all(db.get_ref())
        .await;

    match result {
        Ok(attempts) => {
            if attempts.is_empty() {
                HttpResponse::NotFound().body("No attempts found")
            } else {
                HttpResponse::Ok().json(attempts)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[post("/quiz-attempts")]
pub async fn create_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateAttempt>
) -> impl Responder {
    let data = body.into_inner();
    let new_attempt = QuizAttemptActiveModel {
        quiz_id: Set(data.quiz_id),
        user_id: Set(data.user_id),
        ..Default::default()
    };

    match new_attempt.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("New attempt created successfully!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err)),
    }
}

#[put("/quiz-attempts/{attempt_id}/submit")]
pub async fn submit_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    let attempt_id = path.into_inner();

    let existing = QuizAttemptEntity::find_by_id(attempt_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(attempt)) => {
            let mut active: QuizAttemptActiveModel = attempt.into();

            let now: NaiveDateTime = Local::now().naive_local();
            active.submitted_at = Set(Some(now));

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok()
                    .body(format!("Attempt {} marked as submitted", attempt_id)),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[put("/quiz-attempts/{attempt_id}/grade")]
pub async fn grade_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<MarkAttempt>,
) -> impl Responder {
    let attempt_id = path.into_inner();
    let data = body.into_inner();

    let existing = QuizAttemptEntity::find_by_id(attempt_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(attempt)) => {
            let mut active: QuizAttemptActiveModel = attempt.into();

            if let Some(total_score) = data.total_score {
                active.total_score = Set(Some(total_score));
            }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok()
                    .body(format!("Score for attempt {} updated", attempt_id)),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[delete("/quiz-attempts/{attempt_id}")]
pub async fn delete_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let attempt_id = path.into_inner();
    let existing = QuizAttemptEntity::find_by_id(attempt_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(target_attempt)) => {
            let active_model: QuizAttemptActiveModel = target_attempt.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Attempt deleted!"),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Delete error {}", err)),
    }
}