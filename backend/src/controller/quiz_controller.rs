use actix_session::Session;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::entity::{quiz, roles, user_roles};
use crate::models::quiz::{CreateQuiz, UpdateQuiz};

//SELECT * FROM quiz
#[get("/quiz")]
pub async fn get_quiz(
    db: web::Data<DatabaseConnection>
) -> impl Responder {
    let result = quiz::Entity::find()
    .all(db.get_ref())
    .await;
    match result {
        Ok(quizzes) => {
            if quizzes.is_empty(){
                HttpResponse::NotFound()
                .body("No quizzes found")
            }else{
                HttpResponse::Ok().json(quizzes)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

//SELECT * FROM quiz WHERE course_id =
#[get("/quiz/{course_id}")]
pub async fn get_quiz_by_course_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let course_id = path.into_inner(); 
    let result = quiz::Entity::find()
    .filter(quiz::Column::CourseId.eq(course_id))
    .all(db.get_ref())
    .await;
    match result {
        Ok(quizzes) => {
            if quizzes.is_empty(){
                HttpResponse::NotFound()
                .body("No quizzes found")
            }else{
                HttpResponse::Ok().json(quizzes)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[post("/quiz")]
pub async fn create_quiz(
    db:web::Data<DatabaseConnection>, 
    body:web::Json<CreateQuiz>
) -> impl Responder {
    let data = body.into_inner(); 
    let new_quiz = quiz::ActiveModel{
        course_id: Set(data.course_id),
        title: Set(data.title),
        description: Set(data.description),
        max_attempts: Set(data.max_attempts), 
        time_limit: Set(data.time_limit),
        starts_at: Set(data.starts_at),
        ..Default::default()
    };
    match new_quiz.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok()
        .body("New quiz created successfully!"), 
        Err(err) => HttpResponse::InternalServerError()
        .body(format!("Insert error: {}", err))
    }
}