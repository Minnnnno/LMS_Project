use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::quiz_answers::{
    ActiveModel as QuizAnswerActiveModel, Column as QuizAnswerColumn, Entity as QuizAnswerEntity,
};
use crate::entity::quiz_attempts::{Entity as QuizAttemptEntity, Model as QuizAttemptModel};
use crate::entity::quiz_options::{Column as QuizOptionColumn, Entity as QuizOptionEntity};
use crate::entity::quiz_questions::{Entity as QuizQuestionEntity, QuestionType};
use crate::models::quiz_answers::{GradeQuizAnswer, SubmitLongAnswer, SubmitMcqAnswer};
use crate::services::auth_helpers::{get_role_ids, get_user_id, is_student_only};

fn require_staff(session: &Session, action: &str) -> Result<(), HttpResponse> {
    let role_ids = get_role_ids(session);
    if role_ids.is_empty() {
        return Err(HttpResponse::Unauthorized().body("You must be logged in"));
    }
    if is_student_only(&role_ids) {
        return Err(HttpResponse::Forbidden().body(format!("Students cannot {}", action)));
    }
    Ok(())
}

async fn require_attempt_access(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
    forbidden_message: &str,
) -> Result<(), HttpResponse> {
    let user_id = get_user_id(session)?;
    let role_ids = get_role_ids(session);

    if is_student_only(&role_ids) {
        match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
            Ok(Some(attempt)) if attempt.user_id == user_id => Ok(()),
            Ok(Some(_)) => Err(HttpResponse::Forbidden().body(forbidden_message.to_string())),
            Ok(None) => Err(HttpResponse::NotFound().body("Attempt not found")),
            Err(err) => Err(HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err))),
        }
    } else {
        Ok(())
    }
}

async fn load_accessible_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
    forbidden_message: &str,
) -> Result<QuizAttemptModel, HttpResponse> {
    let user_id = get_user_id(session)?;
    let role_ids = get_role_ids(session);

    match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => {
            if is_student_only(&role_ids) && attempt.user_id != user_id {
                Err(HttpResponse::Forbidden().body(forbidden_message.to_string()))
            } else {
                Ok(attempt)
            }
        }
        Ok(None) => Err(HttpResponse::NotFound().body("Attempt not found")),
        Err(err) => Err(HttpResponse::InternalServerError().body(format!("Database error: {}", err))),
    }
}

fn ensure_attempt_is_open(attempt: &QuizAttemptModel) -> Result<(), HttpResponse> {
    if attempt.submitted_at.is_some() {
        Err(HttpResponse::BadRequest().body("This attempt has already been submitted"))
    } else {
        Ok(())
    }
}

pub async fn list_answers(db: &DatabaseConnection, session: &Session) -> HttpResponse {
    if let Err(response) = require_staff(session, "view all answers") {
        return response;
    }

    match QuizAnswerEntity::find().all(db).await {
        Ok(answers) if answers.is_empty() => HttpResponse::NotFound().body("No quiz answers found"),
        Ok(answers) => HttpResponse::Ok().json(answers),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_answers_by_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
) -> HttpResponse {
    if let Err(response) = require_attempt_access(
        db,
        session,
        attempt_id,
        "You can only view answers for your own attempts",
    ).await {
        return response;
    }

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

pub async fn submit_mcq_answer(
    db: &DatabaseConnection,
    session: &Session,
    data: SubmitMcqAnswer,
) -> HttpResponse {
    let attempt = match load_accessible_attempt(
        db,
        session,
        data.attempt_id,
        "You can only submit answers for your own attempts",
    ).await {
        Ok(attempt) => attempt,
        Err(response) => return response,
    };

    if let Err(response) = ensure_attempt_is_open(&attempt) {
        return response;
    }

    let question = match QuizQuestionEntity::find_by_id(data.question_id).one(db).await {
        Ok(Some(question)) => question,
        Ok(None) => return HttpResponse::NotFound().body("Question not found"),
        Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    };

    if question.question_type != QuestionType::Mcq {
        return HttpResponse::BadRequest()
            .body("This question is not an MCQ. Use /quiz-answers/long-answer instead.");
    }

    if question.quiz_id != attempt.quiz_id {
        return HttpResponse::BadRequest().body("Question does not belong to this attempt's quiz");
    }

    let option = match QuizOptionEntity::find()
        .filter(QuizOptionColumn::OptionId.eq(data.selected_option_id))
        .filter(QuizOptionColumn::QuestionId.eq(data.question_id))
        .one(db)
        .await
    {
        Ok(Some(option)) => option,
        Ok(None) => return HttpResponse::BadRequest().body("Selected option does not belong to this question"),
        Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    };

    if let Ok(Some(existing)) = QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(data.attempt_id))
        .filter(QuizAnswerColumn::QuestionId.eq(data.question_id))
        .one(db)
        .await
    {
        let mut active: QuizAnswerActiveModel = existing.into();
        active.selected_option_id = Set(Some(data.selected_option_id));
        active.answer_text = Set(None);
        active.score = Set(Some(if option.is_correct { question.points } else { 0 }));
        active.feedback = Set(None);

        return match active.update(db).await {
            Ok(_) => HttpResponse::Ok().body("MCQ answer submitted successfully!"),
            Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
        };
    }

    let answer = QuizAnswerActiveModel {
        attempt_id: Set(data.attempt_id),
        question_id: Set(data.question_id),
        selected_option_id: Set(Some(data.selected_option_id)),
        answer_text: Set(None),
        score: Set(Some(if option.is_correct { question.points } else { 0 })),
        feedback: Set(None),
        ..Default::default()
    };

    match answer.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("MCQ answer submitted successfully!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn submit_long_answer(
    db: &DatabaseConnection,
    session: &Session,
    data: SubmitLongAnswer,
) -> HttpResponse {
    let attempt = match load_accessible_attempt(
        db,
        session,
        data.attempt_id,
        "You can only submit answers for your own attempts",
    ).await {
        Ok(attempt) => attempt,
        Err(response) => return response,
    };

    if let Err(response) = ensure_attempt_is_open(&attempt) {
        return response;
    }

    let question = match QuizQuestionEntity::find_by_id(data.question_id).one(db).await {
        Ok(Some(question)) => question,
        Ok(None) => return HttpResponse::NotFound().body("Question not found"),
        Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    };

    if question.question_type != QuestionType::LongAnswer {
        return HttpResponse::BadRequest()
            .body("This question is not a long answer question. Use /quiz-answers/mcq instead.");
    }

    if question.quiz_id != attempt.quiz_id {
        return HttpResponse::BadRequest().body("Question does not belong to this attempt's quiz");
    }

    if data.answer_text.trim().is_empty() {
        return HttpResponse::BadRequest().body("Answer text cannot be empty");
    }

    if let Ok(Some(existing)) = QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(data.attempt_id))
        .filter(QuizAnswerColumn::QuestionId.eq(data.question_id))
        .one(db)
        .await
    {
        let mut active: QuizAnswerActiveModel = existing.into();
        active.selected_option_id = Set(None);
        active.answer_text = Set(Some(data.answer_text));
        active.score = Set(None);
        active.feedback = Set(None);

        return match active.update(db).await {
            Ok(_) => HttpResponse::Ok().body("Long answer submitted successfully!"),
            Err(err) => HttpResponse::InternalServerError().body(format!("Update error: {}", err)),
        };
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
    session: &Session,
    answer_id: i32,
    data: GradeQuizAnswer,
) -> HttpResponse {
    if let Err(response) = require_staff(session, "grade answers") {
        return response;
    }

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

pub async fn delete_answer(
    db: &DatabaseConnection,
    session: &Session,
    answer_id: i32,
) -> HttpResponse {
    if let Err(response) = require_staff(session, "delete answers") {
        return response;
    }

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
