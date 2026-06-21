use actix_session::Session;
use actix_web::HttpResponse;
use chrono::{Duration, Local};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, Set, Statement, TransactionTrait,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::entity::courses;
use crate::entity::quiz::Entity as QuizEntity;
use crate::entity::quiz_answers::{
    ActiveModel as QuizAnswerActiveModel, Column as QuizAnswerColumn, Entity as QuizAnswerEntity,
};
use crate::entity::quiz_attempts::{
    ActiveModel as QuizAttemptActiveModel, Entity as QuizAttemptEntity, Model as QuizAttemptModel,
};
use crate::entity::quiz_options::{Column as QuizOptionColumn, Entity as QuizOptionEntity};
use crate::entity::quiz_questions::{
    Column as QuizQuestionColumn, Entity as QuizQuestionEntity, QuestionType,
};
use crate::models::quiz_answers::{
    AutosaveQuizAnswers, GradeQuizAnswer, SubmitLongAnswer, SubmitMcqAnswer,
};
use crate::services::auth_helpers::{get_role_ids, get_user_id, has_staff_role, is_student_only};
use crate::services::course_service::can_manage_course;

const AUTO_SUBMIT_GRACE_SECONDS: i64 = 30;

#[derive(Serialize)]
struct SavedQuizAnswer {
    question_id: i32,
    selected_option_id: Option<i32>,
    answer_text: Option<String>,
}

fn saved_answer_payload(answers: Vec<crate::entity::quiz_answers::Model>) -> Vec<SavedQuizAnswer> {
    answers
        .into_iter()
        .map(|answer| SavedQuizAnswer {
            question_id: answer.question_id,
            selected_option_id: answer.selected_option_id,
            answer_text: answer.answer_text,
        })
        .collect()
}

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

async fn require_can_manage_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt: &QuizAttemptModel,
    forbidden_message: &str,
) -> Result<(), HttpResponse> {
    let quiz = QuizEntity::find_by_id(attempt.quiz_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Quiz not found"))?;
    let course = courses::Entity::find_by_id(quiz.course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    match can_manage_course(db, session, &course).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(HttpResponse::Forbidden().body(forbidden_message.to_string())),
        Err(response) => Err(response),
    }
}

async fn require_attempt_access(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
    forbidden_message: &str,
) -> Result<(), HttpResponse> {
    let user_id = get_user_id(session)?;
    let role_ids = get_role_ids(session);

    let attempt = QuizAttemptEntity::find_by_id(attempt_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Attempt not found"))?;

    if is_student_only(&role_ids) {
        return if attempt.user_id == user_id {
            Ok(())
        } else {
            Err(HttpResponse::Forbidden().body(forbidden_message.to_string()))
        };
    }

    if has_staff_role(&role_ids) {
        return require_can_manage_attempt(db, session, &attempt, forbidden_message).await;
    }

    Err(HttpResponse::Forbidden().body(forbidden_message.to_string()))
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
        Err(err) => {
            Err(HttpResponse::InternalServerError().body(format!("Database error: {}", err)))
        }
    }
}

fn ensure_student_can_submit_answers(session: &Session) -> Result<(), HttpResponse> {
    let role_ids = get_role_ids(session);

    if has_staff_role(&role_ids) {
        return Err(HttpResponse::Forbidden()
            .body("Instructors and admins can view quiz questions but cannot submit answers"));
    }

    if !is_student_only(&role_ids) {
        return Err(HttpResponse::Forbidden().body("Student role required to submit quiz answers"));
    }

    Ok(())
}

async fn ensure_attempt_accepts_answers(
    db: &impl sea_orm::ConnectionTrait,
    attempt: &QuizAttemptModel,
) -> Result<(), HttpResponse> {
    if attempt.submitted_at.is_some() {
        return Err(HttpResponse::BadRequest().body("This attempt has already been submitted"));
    }

    let quiz = QuizEntity::find_by_id(attempt.quiz_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Quiz not found"))?;

    if let Some(minutes) = quiz.time_limit {
        let expires_at = attempt.started_at + Duration::minutes(minutes as i64);
        let cutoff = expires_at + Duration::seconds(AUTO_SUBMIT_GRACE_SECONDS);

        if Local::now().naive_local() > cutoff {
            return Err(HttpResponse::BadRequest().body("The time limit for this quiz has ended"));
        }
    }

    Ok(())
}

async fn lock_attempt_mutation(
    db: &impl ConnectionTrait,
    attempt_id: i32,
) -> Result<(), HttpResponse> {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1, $2)",
        [1.into(), attempt_id.into()],
    ))
    .await
    .map(|_| ())
    .map_err(|err| HttpResponse::InternalServerError().body(format!("Attempt lock error: {}", err)))
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
    )
    .await
    {
        return response;
    }

    match QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .all(db)
        .await
    {
        Ok(answers) => HttpResponse::Ok().json(saved_answer_payload(answers)),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn autosave_answers(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
    data: AutosaveQuizAnswers,
) -> HttpResponse {
    if let Err(response) = ensure_student_can_submit_answers(session) {
        return response;
    }

    let attempt = match load_accessible_attempt(
        db,
        session,
        attempt_id,
        "You can only save answers for your own attempts",
    )
    .await
    {
        Ok(attempt) => attempt,
        Err(response) => return response,
    };

    if let Err(response) = ensure_attempt_accepts_answers(db, &attempt).await {
        return response;
    }

    let questions = match QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(attempt.quiz_id))
        .all(db)
        .await
    {
        Ok(questions) => questions,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };
    let questions_by_id = questions
        .into_iter()
        .map(|question| (question.question_id, question))
        .collect::<HashMap<_, _>>();
    let mut submitted_question_ids = HashSet::new();
    let mut validated_answers = Vec::with_capacity(data.answers.len());

    for answer in data.answers {
        if !submitted_question_ids.insert(answer.question_id) {
            return HttpResponse::BadRequest().body("Each question may only appear once");
        }

        let question = match questions_by_id.get(&answer.question_id) {
            Some(question) => question,
            None => {
                return HttpResponse::BadRequest()
                    .body("Question does not belong to this attempt's quiz");
            }
        };

        match question.question_type {
            QuestionType::Mcq => {
                if answer
                    .answer_text
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty())
                {
                    return HttpResponse::BadRequest()
                        .body("MCQ answers cannot contain answer text");
                }

                let option = match answer.selected_option_id {
                    Some(option_id) => match QuizOptionEntity::find()
                        .filter(QuizOptionColumn::OptionId.eq(option_id))
                        .filter(QuizOptionColumn::QuestionId.eq(answer.question_id))
                        .one(db)
                        .await
                    {
                        Ok(Some(option)) => Some(option),
                        Ok(None) => {
                            return HttpResponse::BadRequest()
                                .body("Selected option does not belong to this question");
                        }
                        Err(err) => {
                            return HttpResponse::InternalServerError()
                                .body(format!("Database error: {}", err));
                        }
                    },
                    None => None,
                };

                validated_answers.push((
                    answer.question_id,
                    answer.selected_option_id,
                    None,
                    option.map(|option| {
                        if option.is_correct {
                            question.points
                        } else {
                            0
                        }
                    }),
                ));
            }
            QuestionType::LongAnswer => {
                if answer.selected_option_id.is_some() {
                    return HttpResponse::BadRequest()
                        .body("Long answers cannot contain a selected option");
                }

                let answer_text = answer.answer_text.filter(|text| !text.trim().is_empty());
                validated_answers.push((answer.question_id, None, answer_text, None));
            }
        }
    }

    if submitted_question_ids.len() != questions_by_id.len() {
        return HttpResponse::BadRequest().body("Autosave must include every quiz question");
    }

    let transaction = match db.begin().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Could not start autosave: {}", err));
        }
    };

    if let Err(response) = lock_attempt_mutation(&transaction, attempt_id).await {
        let _ = transaction.rollback().await;
        return response;
    }

    let locked_attempt = match QuizAttemptEntity::find_by_id(attempt_id)
        .one(&transaction)
        .await
    {
        Ok(Some(attempt)) => attempt,
        Ok(None) => {
            let _ = transaction.rollback().await;
            return HttpResponse::NotFound().body("Attempt not found");
        }
        Err(err) => {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };
    if let Err(response) = ensure_attempt_accepts_answers(&transaction, &locked_attempt).await {
        let _ = transaction.rollback().await;
        return response;
    }

    for (question_id, selected_option_id, answer_text, score) in validated_answers {
        let existing = match QuizAnswerEntity::find()
            .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
            .filter(QuizAnswerColumn::QuestionId.eq(question_id))
            .one(&transaction)
            .await
        {
            Ok(existing) => existing,
            Err(err) => {
                let _ = transaction.rollback().await;
                return HttpResponse::InternalServerError()
                    .body(format!("Database error: {}", err));
            }
        };

        if selected_option_id.is_none() && answer_text.is_none() {
            if let Some(existing) = existing {
                let active: QuizAnswerActiveModel = existing.into();
                if let Err(err) = active.delete(&transaction).await {
                    let _ = transaction.rollback().await;
                    return HttpResponse::InternalServerError()
                        .body(format!("Autosave delete error: {}", err));
                }
            }
            continue;
        }

        let result = if let Some(existing) = existing {
            let mut active: QuizAnswerActiveModel = existing.into();
            active.selected_option_id = Set(selected_option_id);
            active.answer_text = Set(answer_text);
            active.score = Set(score);
            active.feedback = Set(None);
            active.update(&transaction).await.map(|_| ())
        } else {
            QuizAnswerActiveModel {
                attempt_id: Set(attempt_id),
                question_id: Set(question_id),
                selected_option_id: Set(selected_option_id),
                answer_text: Set(answer_text),
                score: Set(score),
                feedback: Set(None),
                ..Default::default()
            }
            .insert(&transaction)
            .await
            .map(|_| ())
        };

        if let Err(err) = result {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError().body(format!("Autosave error: {}", err));
        }
    }

    if let Err(err) = transaction.commit().await {
        return HttpResponse::InternalServerError().body(format!("Autosave commit error: {}", err));
    }

    match QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .all(db)
        .await
    {
        Ok(answers) => HttpResponse::Ok().json(saved_answer_payload(answers)),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn submit_mcq_answer(
    db: &DatabaseConnection,
    session: &Session,
    data: SubmitMcqAnswer,
) -> HttpResponse {
    if let Err(response) = ensure_student_can_submit_answers(session) {
        return response;
    }

    let attempt = match load_accessible_attempt(
        db,
        session,
        data.attempt_id,
        "You can only submit answers for your own attempts",
    )
    .await
    {
        Ok(attempt) => attempt,
        Err(response) => return response,
    };

    if let Err(response) = ensure_attempt_accepts_answers(db, &attempt).await {
        return response;
    }

    let question = match QuizQuestionEntity::find_by_id(data.question_id)
        .one(db)
        .await
    {
        Ok(Some(question)) => question,
        Ok(None) => return HttpResponse::NotFound().body("Question not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
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
        Ok(None) => {
            return HttpResponse::BadRequest()
                .body("Selected option does not belong to this question");
        }
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
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
        active.score = Set(Some(if option.is_correct {
            question.points
        } else {
            0
        }));
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
        score: Set(Some(if option.is_correct {
            question.points
        } else {
            0
        })),
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
    if let Err(response) = ensure_student_can_submit_answers(session) {
        return response;
    }

    let attempt = match load_accessible_attempt(
        db,
        session,
        data.attempt_id,
        "You can only submit answers for your own attempts",
    )
    .await
    {
        Ok(attempt) => attempt,
        Err(response) => return response,
    };

    if let Err(response) = ensure_attempt_accepts_answers(db, &attempt).await {
        return response;
    }

    let question = match QuizQuestionEntity::find_by_id(data.question_id)
        .one(db)
        .await
    {
        Ok(Some(question)) => question,
        Ok(None) => return HttpResponse::NotFound().body("Question not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
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
            let question = match QuizQuestionEntity::find_by_id(record.question_id)
                .one(db)
                .await
            {
                Ok(Some(question)) => question,
                Ok(None) => return HttpResponse::NotFound().body("Question not found"),
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };

            if question.question_type != QuestionType::LongAnswer {
                return HttpResponse::BadRequest()
                    .body("Only short answer questions can be manually graded");
            }

            if data.score < 0 {
                return HttpResponse::BadRequest().body("Score must be 0 or higher");
            }

            if data.score > question.points {
                return HttpResponse::BadRequest().body(format!(
                    "Marks awarded exceed how much this question is worth ({})",
                    question.points
                ));
            }

            let quiz = match QuizEntity::find_by_id(question.quiz_id).one(db).await {
                Ok(Some(quiz)) => quiz,
                Ok(None) => return HttpResponse::NotFound().body("Quiz not found"),
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };

            let course = match courses::Entity::find_by_id(quiz.course_id).one(db).await {
                Ok(Some(course)) => course,
                Ok(None) => return HttpResponse::NotFound().body("Course not found"),
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };

            match can_manage_course(db, session, &course).await {
                Ok(true) => {}
                Ok(false) => {
                    return HttpResponse::Forbidden().body("You cannot grade this quiz answer");
                }
                Err(response) => return response,
            }

            let attempt_id = record.attempt_id;
            let mut active: QuizAnswerActiveModel = record.into();
            active.score = Set(Some(data.score));
            active.feedback = Set(Some(data.feedback));

            if let Err(err) = active.update(db).await {
                return HttpResponse::InternalServerError().body(format!("Update error: {}", err));
            }

            let answers = match QuizAnswerEntity::find()
                .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
                .all(db)
                .await
            {
                Ok(answers) => answers,
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };

            let total_score = answers
                .iter()
                .filter_map(|answer| answer.score)
                .sum::<i32>();

            let long_answer_questions = match QuizQuestionEntity::find()
                .filter(QuizQuestionColumn::QuizId.eq(question.quiz_id))
                .filter(QuizQuestionColumn::QuestionType.eq(QuestionType::LongAnswer))
                .all(db)
                .await
            {
                Ok(questions) => questions,
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };

            let is_graded = long_answer_questions.iter().all(|question| {
                answers
                    .iter()
                    .find(|answer| answer.question_id == question.question_id)
                    .map(|answer| answer.score.is_some())
                    .unwrap_or(true)
            });

            match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
                Ok(Some(attempt)) => {
                    let mut active_attempt: QuizAttemptActiveModel = attempt.into();
                    active_attempt.total_score = Set(Some(total_score));
                    active_attempt.is_graded = Set(is_graded);

                    match active_attempt.update(db).await {
                        Ok(_) => HttpResponse::Ok()
                            .body(format!("Answer {} graded successfully!", answer_id)),
                        Err(err) => HttpResponse::InternalServerError()
                            .body(format!("Attempt score update error: {}", err)),
                    }
                }
                Ok(None) => HttpResponse::NotFound().body("Attempt not found"),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Database error: {}", err))
                }
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
            let attempt = match QuizAttemptEntity::find_by_id(record.attempt_id)
                .one(db)
                .await
            {
                Ok(Some(attempt)) => attempt,
                Ok(None) => return HttpResponse::NotFound().body("Attempt not found"),
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Database error: {}", err));
                }
            };
            if let Err(response) = require_can_manage_attempt(
                db,
                session,
                &attempt,
                "You cannot delete this quiz answer",
            )
            .await
            {
                return response;
            }

            let transaction = match db.begin().await {
                Ok(transaction) => transaction,
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Could not start answer deletion: {}", err));
                }
            };
            if let Err(response) = lock_attempt_mutation(&transaction, attempt.attempt_id).await {
                let _ = transaction.rollback().await;
                return response;
            }
            let active_model: QuizAnswerActiveModel = record.into();
            match active_model.delete(&transaction).await {
                Ok(_) => match transaction.commit().await {
                    Ok(_) => HttpResponse::Ok().body("Answer deleted!"),
                    Err(err) => HttpResponse::InternalServerError()
                        .body(format!("Answer deletion transaction error: {}", err)),
                },
                Err(err) => {
                    let _ = transaction.rollback().await;
                    HttpResponse::InternalServerError().body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Answer not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error: {}", err)),
    }
}
