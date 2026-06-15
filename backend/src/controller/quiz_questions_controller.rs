use actix_session::Session;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::entity::quiz_questions::{
    Entity as QuizQuestionEntity,
    Column as QuizQuestionColumn,
    ActiveModel as QuizQuestionActiveModel,
};
use crate::entity::quiz_questions::QuestionType;
use crate::models::quiz_questions::{CreateQuizQuestion, UpdateQuizQuestion};
use crate::services::auth_helpers::{get_user_id, get_role_ids, is_student_only};

#[get("/quiz-questions")]
pub async fn get_quiz_questions(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    if get_user_id(&session).is_err() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }

    match QuizQuestionEntity::find().all(db.get_ref()).await {
        Ok(quiz_questions) => {
            if quiz_questions.is_empty() {
                HttpResponse::NotFound().body("No quiz questions found")
            } else {
                HttpResponse::Ok().json(quiz_questions)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[get("/quiz-questions/{quiz_id}")]
pub async fn get_qns_by_quiz_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    if get_user_id(&session).is_err() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }

    let quiz_id = path.into_inner();
    match QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(quiz_id))
        .all(db.get_ref())
        .await
    {
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
    body: web::Json<CreateQuizQuestion>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot create questions");
    }

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
    body: web::Json<UpdateQuizQuestion>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot update questions");
    }

    let question_id = path.into_inner();
    let data = body.into_inner();

    match QuizQuestionEntity::find_by_id(question_id).one(db.get_ref()).await {
        Ok(Some(updated_quiz_qn)) => {
            let mut active: QuizQuestionActiveModel = updated_quiz_qn.into();

            if let Some(question_type) = data.question_type { active.question_type = Set(question_type); }
            if let Some(question_text) = data.question_text { active.question_text = Set(question_text); }
            if let Some(position) = data.position { active.position = Set(position); }
            if let Some(points) = data.points { active.points = Set(points); }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body(format!("Question with id {} updated!", question_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Question not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[delete("/quiz-questions/{question_id}")]
pub async fn delete_quiz_qn(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot delete questions");
    }

    let question_id = path.into_inner();
    match QuizQuestionEntity::find_by_id(question_id).one(db.get_ref()).await {
        Ok(Some(target)) => {
            let active: QuizQuestionActiveModel = target.into();
            match active.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Question deleted!"),
                Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Question not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
    }
}