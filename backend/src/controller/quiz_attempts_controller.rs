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
use crate::models::quiz_attempts::{CreateAttempt, MarkAttempt, SubmitAttempt};
use crate::services::auth_helpers::{get_user_id, get_role_ids, is_student_only};

// Staff only — see all attempts
#[get("/quiz-attempts")]
pub async fn get_quiz_attempts(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot view all attempts");
    }

    match QuizAttemptEntity::find().all(db.get_ref()).await {
        Ok(attempts) => {
            if attempts.is_empty() {
                HttpResponse::NotFound().body("No quiz attempts found")
            } else {
                HttpResponse::Ok().json(attempts)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

// Staff only — see all attempts for a quiz
#[get("/quiz-attempts/quiz/{quiz_id}")]
pub async fn get_attempts_by_quiz_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot view attempts by quiz");
    }

    let quiz_id = path.into_inner();
    match QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::QuizId.eq(quiz_id))
        .all(db.get_ref())
        .await
    {
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

// Students see only their own attempts
#[get("/quiz-attempts/my")]
pub async fn get_my_attempts(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let user_id = match get_user_id(&session) {
        Ok(id) => id,
        Err(res) => return res,
    };

    match QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::UserId.eq(user_id))
        .all(db.get_ref())
        .await
    {
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

// Students create their own attempts; user_id is pulled from session, not the request body
#[post("/quiz-attempts")]
pub async fn create_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateAttempt>,
    session: Session,
) -> impl Responder {
    let user_id = match get_user_id(&session) {
        Ok(id) => id,
        Err(res) => return res,
    };

    let data = body.into_inner();
    let new_attempt = QuizAttemptActiveModel {
        quiz_id: Set(data.quiz_id),
        user_id: Set(user_id), // always from session, never trust the body
        ..Default::default()
    };

    match new_attempt.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("New attempt created successfully!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err)),
    }
}

// Students can only submit their own attempt
#[put("/quiz-attempts/{attempt_id}/submit")]
pub async fn submit_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    let user_id = match get_user_id(&session) {
        Ok(id) => id,
        Err(res) => return res,
    };

    let attempt_id = path.into_inner();
    match QuizAttemptEntity::find_by_id(attempt_id).one(db.get_ref()).await {
        Ok(Some(attempt)) => {
            // students can only submit their own attempt
            if attempt.user_id != user_id && is_student_only(&get_role_ids(&session)) {
                return HttpResponse::Forbidden().body("You can only submit your own attempt");
            }

            let mut active: QuizAttemptActiveModel = attempt.into();
            active.submitted_at = Set(Some(Local::now().naive_local()));

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body(format!("Attempt {} submitted", attempt_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

// Staff only — grade an attempt
#[put("/quiz-attempts/{attempt_id}/grade")]
pub async fn grade_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<MarkAttempt>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot grade attempts");
    }

    let attempt_id = path.into_inner();
    let data = body.into_inner();

    match QuizAttemptEntity::find_by_id(attempt_id).one(db.get_ref()).await {
        Ok(Some(attempt)) => {
            let mut active: QuizAttemptActiveModel = attempt.into();
            if let Some(total_score) = data.total_score {
                active.total_score = Set(Some(total_score));
            }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body(format!("Score for attempt {} updated", attempt_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

// Staff only — delete
#[delete("/quiz-attempts/{attempt_id}")]
pub async fn delete_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot delete attempts");
    }

    let attempt_id = path.into_inner();
    match QuizAttemptEntity::find_by_id(attempt_id).one(db.get_ref()).await {
        Ok(Some(target)) => {
            let active: QuizAttemptActiveModel = target.into();
            match active.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Attempt deleted!"),
                Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Attempt not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
    }
}