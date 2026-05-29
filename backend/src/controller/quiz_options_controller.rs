use actix_session::Session;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::entity::quiz_options::{
    Entity as QuizOptionEntity, 
    Column as QuizOptionColumn,
    ActiveModel as QuizOptionActiveModel,
};
use crate::entity::{roles, user_roles};
use crate::models::quiz_options::{CreateQuizOption, UpdateQuizOption};

//SELECT * FROM quiz
#[get("/quiz-options")]
pub async fn get_quiz_options(
    db: web::Data<DatabaseConnection>
) -> impl Responder {
    let result = QuizOptionEntity::find()
    .all(db.get_ref())
    .await;
    match result {
        Ok(quiz_options) => {
            if quiz_options.is_empty(){
                HttpResponse::NotFound()
                .body("No quiz options found")
            }else{
                HttpResponse::Ok().json(quiz_options)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

// SELECT * FROM quiz WHERE question_id =
#[get("/quiz-options/{option_id}")]
pub async fn get_options_by_qn_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let question_id = path.into_inner();
    let result = QuizOptionEntity::find()
        .filter(QuizOptionColumn::QuestionId.eq(question_id))
        .all(db.get_ref())
        .await;

    match result {
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
    body: web::Json<CreateQuizOption>
) -> impl Responder {
    let data = body.into_inner();
    let new_quiz_option = QuizOptionActiveModel {
        question_id: Set(data.question_id),
        option_text: Set(data.option_text),
        is_correct: Set(data.is_correct),
        position: Set(data.position),
        ..Default::default()
    };

    match new_quiz_option.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("New quiz option created successfully!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err)),
    }
}

#[put("/quiz-options/{option_id}")]
pub async fn update_quiz_option(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<UpdateQuizOption>
) -> impl Responder {
    let option_id = path.into_inner();
    let data = body.into_inner();

    let existing = QuizOptionEntity::find_by_id(option_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(updated_quiz_option)) => {
            let mut active: QuizOptionActiveModel = updated_quiz_option.into();

            if let Some(option_text) = data.option_text {
                active.option_text = Set(option_text);
            }
            if let Some(is_correct) = data.is_correct {
                active.is_correct = Set(is_correct);
            }
            if let Some(position) = data.position {
                active.position = Set(position);
            }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok()
                    .body(format!("Option with id {} updated!", option_id)),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Option not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[delete("/quiz-options/{option_id}")]
pub async fn delete_quiz_option(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let option_id = path.into_inner();
    let existing = QuizOptionEntity::find_by_id(option_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(target_option)) => {
            let active_model: QuizOptionActiveModel = target_option.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Option deleted!"),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Option not found!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Delete error {}", err)),
    }
}