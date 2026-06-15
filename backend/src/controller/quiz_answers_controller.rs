use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use actix_session::Session;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::entity::quiz_answers::{
    Entity as QuizAnswerEntity,
    Column as QuizAnswerColumn,
    ActiveModel as QuizAnswerActiveModel,
};
use crate::entity::quiz_attempts::{
    Entity as QuizAttemptEntity,
    Column as QuizAttemptColumn,
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
use crate::services::auth_helpers::{get_user_id, get_role_ids, is_student_only};

// GET /quiz-answers — staff only
#[get("/quiz-answers")]
pub async fn get_quiz_answers(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot view all answers");
    }

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
// staff: see any attempt's answers
// students: only see answers belonging to their own attempt
#[get("/quiz-answers/attempt/{attempt_id}")]
pub async fn get_answers_by_attempt_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    let user_id = match get_user_id(&session) {
        Ok(id) => id,
        Err(res) => return res,
    };
    let role_ids = get_role_ids(&session);
    let attempt_id = path.into_inner();

    // if student, verify they own this attempt
    if is_student_only(&role_ids) {
        match QuizAttemptEntity::find_by_id(attempt_id)
            .one(db.get_ref())
            .await
        {
            Ok(Some(attempt)) => {
                if attempt.user_id != user_id {
                    return HttpResponse::Forbidden()
                        .body("You can only view answers for your own attempts");
                }
            }
            Ok(None) => return HttpResponse::NotFound().body("Attempt not found"),
            Err(err) => return HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err)),
        }
    }

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

// POST /quiz-answers/mcq — logged in users (students submit their own attempt)
#[post("/quiz-answers/mcq")]
pub async fn submit_mcq_answer(
    db: web::Data<DatabaseConnection>,
    body: web::Json<SubmitMcqAnswer>,
    session: Session,
) -> impl Responder {
    let user_id = match get_user_id(&session) {
        Ok(id) => id,
        Err(res) => return res,
    };
    let role_ids = get_role_ids(&session);
    let data = body.into_inner();

    // students can only submit answers for their own attempts
    if is_student_only(&role_ids) {
        match QuizAttemptEntity::find_by_id(data.attempt_id)
            .one(db.get_ref())
            .await
        {
            Ok(Some(attempt)) => {
                if attempt.user_id != user_id {
                    return HttpResponse::Forbidden()
                        .body("You can only submit answers for your own attempts");
                }
            }
            Ok(None) => return HttpResponse::NotFound().body("Attempt not found"),
            Err(err) => return HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err)),
        }
    }

    // check question exists and is MCQ type
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

    // check selected_option_id belongs to this question
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

// POST /quiz-answers/long-answer — logged in users
#[post("/quiz-answers/long-answer")]
pub async fn submit_long_answer(
    db: web::Data<DatabaseConnection>,
    body: web::Json<SubmitLongAnswer>,
    session: Session,
) -> impl Responder {
    let user_id = match get_user_id(&session) {
        Ok(id) => id,
        Err(res) => return res,
    };
    let role_ids = get_role_ids(&session);
    let data = body.into_inner();

    // students can only submit answers for their own attempts
    if is_student_only(&role_ids) {
        match QuizAttemptEntity::find_by_id(data.attempt_id)
            .one(db.get_ref())
            .await
        {
            Ok(Some(attempt)) => {
                if attempt.user_id != user_id {
                    return HttpResponse::Forbidden()
                        .body("You can only submit answers for your own attempts");
                }
            }
            Ok(None) => return HttpResponse::NotFound().body("Attempt not found"),
            Err(err) => return HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err)),
        }
    }

    // check question exists and is long answer type
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

    if data.answer_text.trim().is_empty() {
        return HttpResponse::BadRequest().body("Answer text cannot be empty");
    }

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

// PUT /quiz-answers/{answer_id}/grade — staff only
#[put("/quiz-answers/{answer_id}/grade")]
pub async fn grade_quiz_answer(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<GradeQuizAnswer>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot grade answers");
    }

    let answer_id = path.into_inner();
    let data = body.into_inner();

    match QuizAnswerEntity::find_by_id(answer_id).one(db.get_ref()).await {
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

// DELETE /quiz-answers/{answer_id} — staff only
#[delete("/quiz-answers/{answer_id}")]
pub async fn delete_quiz_answer(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    let role_ids = get_role_ids(&session);
    if role_ids.is_empty() {
        return HttpResponse::Unauthorized().body("You must be logged in");
    }
    if is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Students cannot delete answers");
    }

    let answer_id = path.into_inner();
    match QuizAnswerEntity::find_by_id(answer_id).one(db.get_ref()).await {
        Ok(Some(record)) => {
            let active: QuizAnswerActiveModel = record.into();
            match active.delete(db.get_ref()).await {
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