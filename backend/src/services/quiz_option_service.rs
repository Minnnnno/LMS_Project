use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::quiz_options::{
    ActiveModel as QuizOptionActiveModel, Column as QuizOptionColumn, Entity as QuizOptionEntity,
};
use crate::entity::{courses, quiz, quiz_questions};
use crate::models::quiz_options::{CreateQuizOption, UpdateQuizOption};
use crate::services::auth_helpers::get_user_id;
use crate::services::course_service::can_manage_course;

async fn get_course_for_question(
    db: &DatabaseConnection,
    question_id: i32,
) -> Result<courses::Model, HttpResponse> {
    let question = quiz_questions::Entity::find_by_id(question_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding quiz question: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Quiz question not found"))?;

    let quiz = quiz::Entity::find_by_id(question.quiz_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding quiz: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Quiz not found"))?;

    courses::Entity::find_by_id(quiz.course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))
}

async fn require_can_manage_question(
    db: &DatabaseConnection,
    session: &Session,
    question_id: i32,
) -> Result<(), HttpResponse> {
    let course = get_course_for_question(db, question_id).await?;

    match can_manage_course(db, session, &course).await {
        Ok(true) => Ok(()),
        Ok(false) => {
            Err(HttpResponse::Forbidden().body("You cannot manage options for this question"))
        }
        Err(response) => Err(response),
    }
}

pub async fn list_options(db: &DatabaseConnection, session: &Session) -> HttpResponse {
    if let Err(response) = get_user_id(session) {
        return response;
    }

    match QuizOptionEntity::find().all(db).await {
        Ok(options) if options.is_empty() => HttpResponse::NotFound().body("No quiz options found"),
        Ok(options) => HttpResponse::Ok().json(options),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_options_by_question(
    db: &DatabaseConnection,
    session: &Session,
    question_id: i32,
) -> HttpResponse {
    if let Err(response) = get_user_id(session) {
        return response;
    }

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

pub async fn create_option(
    db: &DatabaseConnection,
    session: &Session,
    data: CreateQuizOption,
) -> HttpResponse {
    if let Err(response) = require_can_manage_question(db, session, data.question_id).await {
        return response;
    }

    let option = QuizOptionActiveModel {
        question_id: Set(data.question_id),
        option_text: Set(data.option_text),
        is_correct: Set(data.is_correct),
        position: Set(data.position),
        ..Default::default()
    };

    match option.insert(db).await {
        Ok(option) => HttpResponse::Ok().json(option),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn update_option(
    db: &DatabaseConnection,
    session: &Session,
    option_id: i32,
    data: UpdateQuizOption,
) -> HttpResponse {
    match QuizOptionEntity::find_by_id(option_id).one(db).await {
        Ok(Some(option)) => {
            if let Err(response) =
                require_can_manage_question(db, session, option.question_id).await
            {
                return response;
            }

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
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Update error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Option not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn delete_option(
    db: &DatabaseConnection,
    session: &Session,
    option_id: i32,
) -> HttpResponse {
    match QuizOptionEntity::find_by_id(option_id).one(db).await {
        Ok(Some(option)) => {
            if let Err(response) =
                require_can_manage_question(db, session, option.question_id).await
            {
                return response;
            }

            let active_model: QuizOptionActiveModel = option.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Option deleted!"),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Option not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
