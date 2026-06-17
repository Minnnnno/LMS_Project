use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::quiz_questions::{
    ActiveModel as QuizQuestionActiveModel, Column as QuizQuestionColumn,
    Entity as QuizQuestionEntity,
};
use crate::entity::{courses, quiz};
use crate::models::quiz_questions::{CreateQuizQuestion, UpdateQuizQuestion};
use crate::services::course_service::can_manage_course;

fn validate_question_fields(
    question_text: Option<&str>,
    position: Option<i32>,
    points: Option<i32>,
) -> Result<(), HttpResponse> {
    if question_text
        .map(|value| value.trim().is_empty())
        .unwrap_or(false)
    {
        return Err(HttpResponse::BadRequest().body("Question text cannot be empty"));
    }

    if position.map(|value| value < 1).unwrap_or(false) {
        return Err(HttpResponse::BadRequest().body("Question position must be 1 or higher"));
    }

    if points.map(|value| value < 1).unwrap_or(false) {
        return Err(HttpResponse::BadRequest().body("Question points must be 1 or higher"));
    }

    Ok(())
}

async fn get_course_for_quiz(
    db: &DatabaseConnection,
    quiz_id: i32,
) -> Result<courses::Model, HttpResponse> {
    let quiz = quiz::Entity::find_by_id(quiz_id)
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

async fn require_can_manage_quiz(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> Result<(), HttpResponse> {
    let course = get_course_for_quiz(db, quiz_id).await?;

    match can_manage_course(db, session, &course).await {
        Ok(true) => Ok(()),
        Ok(false) => {
            Err(HttpResponse::Forbidden().body("You cannot manage questions for this quiz"))
        }
        Err(response) => Err(response),
    }
}

pub async fn list_questions_by_quiz(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> HttpResponse {
    if let Err(response) = require_can_manage_quiz(db, session, quiz_id).await {
        return response;
    }

    match QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(quiz_id))
        .all(db)
        .await
    {
        Ok(questions) if questions.is_empty() => {
            HttpResponse::NotFound().body("No questions found")
        }
        Ok(questions) => HttpResponse::Ok().json(questions),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn create_question(
    db: &DatabaseConnection,
    session: &Session,
    data: CreateQuizQuestion,
) -> HttpResponse {
    if let Err(response) =
        validate_question_fields(Some(&data.question_text), Some(data.position), data.points)
    {
        return response;
    }

    if let Err(response) = require_can_manage_quiz(db, session, data.quiz_id).await {
        return response;
    }

    let question = QuizQuestionActiveModel {
        quiz_id: Set(data.quiz_id),
        question_type: Set(data.question_type),
        question_text: Set(data.question_text),
        position: Set(data.position),
        points: Set(data.points.unwrap_or(1)),
        ..Default::default()
    };

    match question.insert(db).await {
        Ok(question) => HttpResponse::Ok().json(question),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn update_question(
    db: &DatabaseConnection,
    session: &Session,
    question_id: i32,
    data: UpdateQuizQuestion,
) -> HttpResponse {
    if let Err(response) =
        validate_question_fields(data.question_text.as_deref(), data.position, data.points)
    {
        return response;
    }

    match QuizQuestionEntity::find_by_id(question_id).one(db).await {
        Ok(Some(question)) => {
            if let Err(response) = require_can_manage_quiz(db, session, question.quiz_id).await {
                return response;
            }

            let mut active: QuizQuestionActiveModel = question.into();

            if let Some(question_type) = data.question_type {
                active.question_type = Set(question_type);
            }
            if let Some(question_text) = data.question_text {
                active.question_text = Set(question_text);
            }
            if let Some(position) = data.position {
                active.position = Set(position);
            }
            if let Some(points) = data.points {
                active.points = Set(points);
            }

            match active.update(db).await {
                Ok(_) => {
                    HttpResponse::Ok().body(format!("Question with id {} updated!", question_id))
                }
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Update error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Question not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn delete_question(
    db: &DatabaseConnection,
    session: &Session,
    question_id: i32,
) -> HttpResponse {
    match QuizQuestionEntity::find_by_id(question_id).one(db).await {
        Ok(Some(question)) => {
            if let Err(response) = require_can_manage_quiz(db, session, question.quiz_id).await {
                return response;
            }

            let active_model: QuizQuestionActiveModel = question.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Question deleted!"),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Question not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
