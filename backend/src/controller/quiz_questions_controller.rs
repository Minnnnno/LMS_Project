use actix_session::Session;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::entity::quiz_questions::Entity as QuizQuestionEntity;
use crate::entity::{quiz, roles, user_roles};
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
