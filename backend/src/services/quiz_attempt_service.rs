use actix_session::Session;
use actix_web::HttpResponse;
use chrono::Local;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::quiz_attempts::{
    ActiveModel as QuizAttemptActiveModel, Column as QuizAttemptColumn,
    Entity as QuizAttemptEntity,
};
use crate::models::quiz_attempts::{CreateAttempt, MarkAttempt};
use crate::services::auth_helpers::{get_role_ids, get_user_id, is_student_only};

fn require_staff(session: &Session, action: &str) -> Result<(), HttpResponse> {
    let role_ids = get_role_ids(session);
    if role_ids.is_empty() {
        return Err(HttpResponse::Unauthorized().body("You must be logged in"));
    }
    if is_student_only(&role_ids) {
        return Err(HttpResponse::Forbidden().body(format!("Students cannot {}", action)));
    }
    Ok(())
}

pub async fn list_attempts(db: &DatabaseConnection, session: &Session) -> HttpResponse {
    if let Err(response) = require_staff(session, "view all attempts") {
        return response;
    }

    match QuizAttemptEntity::find().all(db).await {
        Ok(attempts) if attempts.is_empty() => HttpResponse::NotFound().body("No quiz attempts found"),
        Ok(attempts) => HttpResponse::Ok().json(attempts),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_attempts_by_quiz(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> HttpResponse {
    if let Err(response) = require_staff(session, "view attempts by quiz") {
        return response;
    }

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

pub async fn list_my_attempts(db: &DatabaseConnection, session: &Session) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::UserId.eq(user_id))
        .all(db)
        .await
    {
        Ok(attempts) if attempts.is_empty() => HttpResponse::NotFound().body("No attempts found"),
        Ok(attempts) => HttpResponse::Ok().json(attempts),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn create_attempt(
    db: &DatabaseConnection,
    session: &Session,
    data: CreateAttempt,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let attempt = QuizAttemptActiveModel {
        quiz_id: Set(data.quiz_id),
        user_id: Set(user_id),
        ..Default::default()
    };

    match attempt.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("New attempt created successfully!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn submit_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => {
            if attempt.user_id != user_id && is_student_only(&get_role_ids(session)) {
                return HttpResponse::Forbidden().body("You can only submit your own attempt");
            }

            let mut active: QuizAttemptActiveModel = attempt.into();
            active.submitted_at = Set(Some(Local::now().naive_local()));

            match active.update(db).await {
                Ok(_) => HttpResponse::Ok().body(format!("Attempt {} submitted", attempt_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn grade_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
    data: MarkAttempt,
) -> HttpResponse {
    if let Err(response) = require_staff(session, "grade attempts") {
        return response;
    }

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

pub async fn delete_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
) -> HttpResponse {
    if let Err(response) = require_staff(session, "delete attempts") {
        return response;
    }

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
