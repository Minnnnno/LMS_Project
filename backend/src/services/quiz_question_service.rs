use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::quiz_questions::{
    ActiveModel as QuizQuestionActiveModel, Column as QuizQuestionColumn,
    Entity as QuizQuestionEntity,
};
use crate::models::quiz_questions::{CreateQuizQuestion, UpdateQuizQuestion};

pub async fn list_questions(db: &DatabaseConnection) -> HttpResponse {
    match QuizQuestionEntity::find().all(db).await {
        Ok(questions) if questions.is_empty() => HttpResponse::NotFound().body("No quiz questions found"),
        Ok(questions) => HttpResponse::Ok().json(questions),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_questions_by_quiz(db: &DatabaseConnection, quiz_id: i32) -> HttpResponse {
    match QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(quiz_id))
        .all(db)
        .await
    {
        Ok(questions) if questions.is_empty() => HttpResponse::NotFound().body("No questions found"),
        Ok(questions) => HttpResponse::Ok().json(questions),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn create_question(db: &DatabaseConnection, data: CreateQuizQuestion) -> HttpResponse {
    let question = QuizQuestionActiveModel {
        quiz_id: Set(data.quiz_id),
        question_type: Set(data.question_type),
        question_text: Set(data.question_text),
        position: Set(data.position),
        points: Set(data.points.unwrap_or(1)),
        ..Default::default()
    };

    match question.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("New quiz question created successfully!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn update_question(
    db: &DatabaseConnection,
    question_id: i32,
    data: UpdateQuizQuestion,
) -> HttpResponse {
    match QuizQuestionEntity::find_by_id(question_id).one(db).await {
        Ok(Some(question)) => {
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
                Ok(_) => HttpResponse::Ok().body(format!("Question with id {} updated!", question_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Question not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn delete_question(db: &DatabaseConnection, question_id: i32) -> HttpResponse {
    match QuizQuestionEntity::find_by_id(question_id).one(db).await {
        Ok(Some(question)) => {
            let active_model: QuizQuestionActiveModel = question.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Question deleted!"),
                Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Question not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
