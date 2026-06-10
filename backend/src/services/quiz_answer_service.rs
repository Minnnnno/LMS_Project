use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::quiz_answers::{
    ActiveModel as QuizAnswerActiveModel, Column as QuizAnswerColumn, Entity as QuizAnswerEntity,
};
use crate::entity::quiz_options::{Column as QuizOptionColumn, Entity as QuizOptionEntity};
use crate::entity::quiz_questions::{Entity as QuizQuestionEntity, QuestionType};
use crate::models::quiz_answers::{GradeQuizAnswer, SubmitLongAnswer, SubmitMcqAnswer};

pub async fn list_answers(db: &DatabaseConnection) -> HttpResponse {
    match QuizAnswerEntity::find().all(db).await {
        Ok(answers) if answers.is_empty() => HttpResponse::NotFound().body("No quiz answers found"),
        Ok(answers) => HttpResponse::Ok().json(answers),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_answers_by_attempt(db: &DatabaseConnection, attempt_id: i32) -> HttpResponse {
    match QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .all(db)
        .await
    {
        Ok(answers) if answers.is_empty() => HttpResponse::NotFound().body("No answers found for this attempt"),
        Ok(answers) => HttpResponse::Ok().json(answers),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn submit_mcq_answer(db: &DatabaseConnection, data: SubmitMcqAnswer) -> HttpResponse {
    let question = match QuizQuestionEntity::find_by_id(data.question_id).one(db).await {
        Ok(Some(question)) => question,
        Ok(None) => return HttpResponse::NotFound().body("Question not found"),
        Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    };

    if question.question_type != QuestionType::Mcq {
        return HttpResponse::BadRequest()
            .body("This question is not an MCQ. Use /quiz-answers/long-answer instead.");
    }

    match QuizOptionEntity::find()
        .filter(QuizOptionColumn::OptionId.eq(data.selected_option_id))
        .filter(QuizOptionColumn::QuestionId.eq(data.question_id))
        .one(db)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => return HttpResponse::BadRequest().body("Selected option does not belong to this question"),
        Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }

    let answer = QuizAnswerActiveModel {
        attempt_id: Set(data.attempt_id),
        question_id: Set(data.question_id),
        selected_option_id: Set(Some(data.selected_option_id)),
        answer_text: Set(None),
        score: Set(None),
        feedback: Set(None),
        ..Default::default()
    };

    match answer.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("MCQ answer submitted successfully!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn submit_long_answer(db: &DatabaseConnection, data: SubmitLongAnswer) -> HttpResponse {
    let question = match QuizQuestionEntity::find_by_id(data.question_id).one(db).await {
        Ok(Some(question)) => question,
        Ok(None) => return HttpResponse::NotFound().body("Question not found"),
        Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    };

    if question.question_type != QuestionType::LongAnswer {
        return HttpResponse::BadRequest()
            .body("This question is not a long answer question. Use /quiz-answers/mcq instead.");
    }

    if data.answer_text.trim().is_empty() {
        return HttpResponse::BadRequest().body("Answer text cannot be empty");
    }

    let answer = QuizAnswerActiveModel {
        attempt_id: Set(data.attempt_id),
        question_id: Set(data.question_id),
        selected_option_id: Set(None),
        answer_text: Set(Some(data.answer_text)),
        score: Set(None),
        feedback: Set(None),
        ..Default::default()
    };

    match answer.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("Long answer submitted successfully!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn grade_answer(
    db: &DatabaseConnection,
    answer_id: i32,
    data: GradeQuizAnswer,
) -> HttpResponse {
    match QuizAnswerEntity::find_by_id(answer_id).one(db).await {
        Ok(Some(record)) => {
            let mut active: QuizAnswerActiveModel = record.into();
            active.score = Set(Some(data.score));
            active.feedback = Set(Some(data.feedback));

            match active.update(db).await {
                Ok(_) => HttpResponse::Ok().body(format!("Answer {} graded successfully!", answer_id)),
                Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Answer not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn delete_answer(db: &DatabaseConnection, answer_id: i32) -> HttpResponse {
    match QuizAnswerEntity::find_by_id(answer_id).one(db).await {
        Ok(Some(record)) => {
            let active_model: QuizAnswerActiveModel = record.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Answer deleted!"),
                Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Answer not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
    }
}
