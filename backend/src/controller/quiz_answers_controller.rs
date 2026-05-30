use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::entity::quiz_answers::{
    Entity as QuizAnswerEntity,
    Column as QuizAnswerColumn,
    ActiveModel as QuizAnswerActiveModel,
};
use crate::entity::quiz_questions::{
    Entity as QuizQuestionEntity,
    Model as QuizQuestionModel,
};
use crate::entity::quiz_options::{
    Entity as QuizOptionEntity,
    Column as QuizOptionColumn,
};
use crate::entity::quiz_questions::QuestionType;
use crate::models::quiz_answers::{SubmitMcqAnswer, SubmitLongAnswer, GradeQuizAnswer};

// GET /quiz-answers
#[get("/quiz-answers")]
pub async fn get_quiz_answers(
    db: web::Data<DatabaseConnection>,
) -> impl Responder {
    match QuizAnswerEntity::find().all(db.get_ref()).await {
        Ok(answers) => {
            if answers.is_empty() {
                HttpResponse::NotFound().body("No quiz answers found")
            } else {
                HttpResponse::Ok().json(answers)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

// GET /quiz-answers/attempt/{attempt_id}
#[get("/quiz-answers/attempt/{attempt_id}")]
pub async fn get_answers_by_attempt_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    let attempt_id = path.into_inner();
    match QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .all(db.get_ref())
        .await
    {
        Ok(answers) => {
            if answers.is_empty() {
                HttpResponse::NotFound().body("No answers found for this attempt")
            } else {
                HttpResponse::Ok().json(answers)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

// POST /quiz-answers/mcq — MCQ submission, validates option belongs to question
#[post("/quiz-answers/mcq")]
pub async fn submit_mcq_answer(
    db: web::Data<DatabaseConnection>,
    body: web::Json<SubmitMcqAnswer>,
) -> impl Responder {
    let data = body.into_inner();

    // 1. check question exists and is actually MCQ type
    let question = match QuizQuestionEntity::find_by_id(data.question_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(q)) => q,
        Ok(None) => return HttpResponse::NotFound().body("Question not found"),
        Err(err) => return HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    };

    if question.question_type != QuestionType::Mcq {
        return HttpResponse::BadRequest()
            .body("This question is not an MCQ. Use /quiz-answers/long-answer instead.");
    }

    // 2. check selected_option_id actually belongs to this question
    let option = match QuizOptionEntity::find()
        .filter(QuizOptionColumn::OptionId.eq(data.selected_option_id))
        .filter(QuizOptionColumn::QuestionId.eq(data.question_id))
        .one(db.get_ref())
        .await
    {
        Ok(Some(o)) => o,
        Ok(None) => return HttpResponse::BadRequest()
            .body("Selected option does not belong to this question"),
        Err(err) => return HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    };

    // 3. insert the answer
    let new_answer = QuizAnswerActiveModel {
        attempt_id: Set(data.attempt_id),
        question_id: Set(data.question_id),
        selected_option_id: Set(Some(data.selected_option_id)),
        answer_text: Set(None),
        score: Set(None),
        feedback: Set(None),
        ..Default::default()
    };

    match new_answer.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("MCQ answer submitted successfully!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err)),
    }
}

// POST /quiz-answers/long-answer — long answer submission
#[post("/quiz-answers/long-answer")]
pub async fn submit_long_answer(
    db: web::Data<DatabaseConnection>,
    body: web::Json<SubmitLongAnswer>,
) -> impl Responder {
    let data = body.into_inner();

    // 1. check question exists and is actually long_answer type
    let question = match QuizQuestionEntity::find_by_id(data.question_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(q)) => q,
        Ok(None) => return HttpResponse::NotFound().body("Question not found"),
        Err(err) => return HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    };

    if question.question_type != QuestionType::LongAnswer {
        return HttpResponse::BadRequest()
            .body("This question is not a long answer question. Use /quiz-answers/mcq instead.");
    }

    // 2. reject empty answers
    if data.answer_text.trim().is_empty() {
        return HttpResponse::BadRequest().body("Answer text cannot be empty");
    }

    // 3. insert the answer
    let new_answer = QuizAnswerActiveModel {
        attempt_id: Set(data.attempt_id),
        question_id: Set(data.question_id),
        selected_option_id: Set(None),
        answer_text: Set(Some(data.answer_text)),
        score: Set(None),
        feedback: Set(None),
        ..Default::default()
    };

    match new_answer.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("Long answer submitted successfully!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err)),
    }
}

// PUT /quiz-answers/{answer_id}/grade
#[put("/quiz-answers/{answer_id}/grade")]
pub async fn grade_quiz_answer(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<GradeQuizAnswer>,
) -> impl Responder {
    let answer_id = path.into_inner();
    let data = body.into_inner();

    match QuizAnswerEntity::find_by_id(answer_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(record)) => {
            let mut active: QuizAnswerActiveModel = record.into();
            active.score = Set(Some(data.score));
            active.feedback = Set(Some(data.feedback));

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok()
                    .body(format!("Answer {} graded successfully!", answer_id)),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Answer not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

// DELETE /quiz-answers/{answer_id}
#[delete("/quiz-answers/{answer_id}")]
pub async fn delete_quiz_answer(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    let answer_id = path.into_inner();
    match QuizAnswerEntity::find_by_id(answer_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(record)) => {
            let active_model: QuizAnswerActiveModel = record.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Answer deleted!"),
                Err(err) => HttpResponse::InternalServerError()
                    .body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Answer not found!"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Delete error: {}", err)),
    }
}