use actix_session::Session;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::entity::quiz_questions::{
    Entity as QuizQuestionEntity, 
    Column as QuizQuestionColumn,
    ActiveModel as QuizQuestionActiveModel,
};
use crate::entity::{roles, user_roles};
use crate::entity::quiz_questions::QuestionType;
use crate::models::quiz_questions::{CreateQuizQuestion, UpdateQuizQuestion};

//SELECT * FROM quiz
#[get("/quiz-questions")]
pub async fn get_quiz_questions(
    db: web::Data<DatabaseConnection>
) -> impl Responder {
    let result = QuizQuestionEntity::find()
    .all(db.get_ref())
    .await;
    match result {
        Ok(quiz_questions) => {
            if quiz_questions.is_empty(){
                HttpResponse::NotFound()
                .body("No quiz questions found")
            }else{
                HttpResponse::Ok().json(quiz_questions)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

// SELECT * FROM quiz WHERE quiz_id =
#[get("/quiz-questions/{quiz_id}")]
pub async fn get_qns_by_quiz_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let quiz_id = path.into_inner();
    let result = QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(quiz_id))
        .all(db.get_ref())
        .await;

    match result {
        Ok(questions) => {
            if questions.is_empty() {
                HttpResponse::NotFound().body("No questions found")
            } else {
                HttpResponse::Ok().json(questions)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[post("/quiz-questions")]
pub async fn create_quiz_qn(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateQuizQuestion>
) -> impl Responder {
    let data = body.into_inner();
    let new_quiz_qn = QuizQuestionActiveModel {
        quiz_id: Set(data.quiz_id),
        question_type: Set(data.question_type),
        question_text: Set(data.question_text),
        position: Set(data.position),
        points: Set(data.points.unwrap_or(1)),
        ..Default::default()
    };

    match new_quiz_qn.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("New quiz question created successfully!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err)),
    }
}

#[put("/quiz-questions/{question_id}")]
pub async fn update_quiz_qn(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<UpdateQuizQuestion>
) -> impl Responder {
    let question_id = path.into_inner();
    let data = body.into_inner();

    let existing = QuizQuestionEntity::find_by_id(question_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(updated_quiz_qn)) => {
            let mut active: QuizQuestionActiveModel = updated_quiz_qn.into();

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

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok()
                    .body(format!("Question with id {} updated!", question_id)),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Question not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[delete("/quiz-questions/{question_id}")]
pub async fn delete_quiz_qn(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let question_id = path.into_inner();
    let existing = QuizQuestionEntity::find_by_id(question_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(target_question)) => {
            let active_model: QuizQuestionActiveModel = target_question.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Question deleted!"),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Question not found!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Delete error {}", err)),
    }
}