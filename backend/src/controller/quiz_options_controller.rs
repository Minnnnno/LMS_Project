use actix_session::Session;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::entity::quiz_options::{
    Entity as QuizOptionEntity,
    Column as QuizOptionColumn,
    ActiveModel as QuizOptionActiveModel,
};
use crate::models::quiz_options::{CreateQuizOption, UpdateQuizOption};
use crate::services::auth_helpers::{get_user_id, get_role_ids, is_student_only};

#[get("/quiz-options")]
pub async fn get_quiz_options(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    if get_user_id(&session).is_err() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }

    match QuizOptionEntity::find().all(db.get_ref()).await {
        Ok(quiz_options) => {
            if quiz_options.is_empty() {
                HttpResponse::NotFound().body("No quiz options found")
            } else {
                HttpResponse::Ok().json(quiz_options)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

// Fixed: route param renamed from {option_id} to {question_id} to match intent
#[get("/quiz-options/by-question/{question_id}")]
pub async fn get_options_by_qn_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    if get_user_id(&session).is_err() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }

    let question_id = path.into_inner();
    match QuizOptionEntity::find()
        .filter(QuizOptionColumn::QuestionId.eq(question_id))
        .all(db.get_ref())
        .await
    {
        Ok(options) => {
            if options.is_empty() {
                HttpResponse::NotFound().body("No options found")
            } else {
                HttpResponse::Ok().json(options)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[post("/quiz-options")]
pub async fn create_quiz_option(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateQuizOption>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot create options");
    }

    let data = body.into_inner();
    let new_option = QuizOptionActiveModel {
        question_id: Set(data.question_id),
        option_text: Set(data.option_text),
        is_correct: Set(data.is_correct),
        position: Set(data.position),
        ..Default::default()
    };

    match new_option.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("New quiz option created successfully!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err)),
    }
}

#[put("/quiz-options/{option_id}")]
pub async fn update_quiz_option(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<UpdateQuizOption>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot update options");
    }

    let option_id = path.into_inner();
    let data = body.into_inner();

    match QuizOptionEntity::find_by_id(option_id).one(db.get_ref()).await {
        Ok(Some(existing)) => {
            let mut active: QuizOptionActiveModel = existing.into();

            if let Some(option_text) = data.option_text { active.option_text = Set(option_text); }
            if let Some(is_correct) = data.is_correct { active.is_correct = Set(is_correct); }
            if let Some(position) = data.position { active.position = Set(position); }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body(format!("Option with id {} updated!", option_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Option not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[delete("/quiz-options/{option_id}")]
pub async fn delete_quiz_option(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot delete options");
    }

    let option_id = path.into_inner();
    match QuizOptionEntity::find_by_id(option_id).one(db.get_ref()).await {
        Ok(Some(target)) => {
            let active: QuizOptionActiveModel = target.into();
            match active.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Option deleted!"),
                Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Option not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
    }
}