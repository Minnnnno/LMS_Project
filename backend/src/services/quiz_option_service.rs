use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::quiz_options::{
    ActiveModel as QuizOptionActiveModel, Column as QuizOptionColumn, Entity as QuizOptionEntity,
};
use crate::models::quiz_options::{CreateQuizOption, UpdateQuizOption};

pub async fn list_options(db: &DatabaseConnection) -> HttpResponse {
    match QuizOptionEntity::find().all(db).await {
        Ok(options) if options.is_empty() => HttpResponse::NotFound().body("No quiz options found"),
        Ok(options) => HttpResponse::Ok().json(options),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_options_by_question(db: &DatabaseConnection, question_id: i32) -> HttpResponse {
    match QuizOptionEntity::find()
        .filter(QuizOptionColumn::QuestionId.eq(question_id))
        .all(db)
        .await
    {
        Ok(options) if options.is_empty() => HttpResponse::NotFound().body("No options found"),
        Ok(options) => HttpResponse::Ok().json(options),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn create_option(db: &DatabaseConnection, data: CreateQuizOption) -> HttpResponse {
    let option = QuizOptionActiveModel {
        question_id: Set(data.question_id),
        option_text: Set(data.option_text),
        is_correct: Set(data.is_correct),
        position: Set(data.position),
        ..Default::default()
    };

    match option.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("New quiz option created successfully!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn update_option(
    db: &DatabaseConnection,
    option_id: i32,
    data: UpdateQuizOption,
) -> HttpResponse {
    match QuizOptionEntity::find_by_id(option_id).one(db).await {
        Ok(Some(option)) => {
            let mut active: QuizOptionActiveModel = option.into();

            if let Some(option_text) = data.option_text {
                active.option_text = Set(option_text);
            }
            if let Some(is_correct) = data.is_correct {
                active.is_correct = Set(is_correct);
            }
            if let Some(position) = data.position {
                active.position = Set(position);
            }

            match active.update(db).await {
                Ok(_) => HttpResponse::Ok().body(format!("Option with id {} updated!", option_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Option not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn delete_option(db: &DatabaseConnection, option_id: i32) -> HttpResponse {
    match QuizOptionEntity::find_by_id(option_id).one(db).await {
        Ok(Some(option)) => {
            let active_model: QuizOptionActiveModel = option.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Option deleted!"),
                Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Option not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
