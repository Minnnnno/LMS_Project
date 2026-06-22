use actix_session::Session;
use actix_web::HttpResponse;
use chrono::{Duration, Local};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use std::collections::{HashMap, HashSet};

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
use crate::models::quiz_answers::{GradeQuizAnswer, SaveQuizAnswers};
use crate::services::auth_helpers::{get_role_ids, get_user_id, is_student_only};
use crate::services::quiz_helper;

const AUTO_SUBMIT_GRACE_SECONDS: i64 = 30;

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

pub async fn save_answers(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
    data: SaveQuizAnswers,
) -> HttpResponse {
    if let Err(response) = quiz_helper::require_student(session) {
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
    let selected_option_ids = data
        .answers
        .iter()
        .filter_map(|answer| answer.selected_option_id)
        .collect::<Vec<_>>();
    let options_by_id = if selected_option_ids.is_empty() {
        HashMap::new()
    } else {
        match QuizOptionEntity::find()
            .filter(QuizOptionColumn::OptionId.is_in(selected_option_ids))
            .all(db)
            .await
        {
            Ok(options) => options
                .into_iter()
                .map(|option| (option.option_id, option))
                .collect::<HashMap<_, _>>(),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error: {}", err));
            }
        }
    };
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
                    Some(option_id) => match options_by_id.get(&option_id) {
                        Some(option) if option.question_id == answer.question_id => Some(option),
                        _ => {
                            return HttpResponse::BadRequest()
                                .body("Selected option does not belong to this question");
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
                .body(format!("Could not start answer save: {}", err));
        }
    };

    if let Err(response) = quiz_helper::lock_attempt(&transaction, attempt_id).await {
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

    let existing_answers = match QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .all(&transaction)
        .await
    {
        Ok(answers) => answers,
        Err(err) => {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };
    let mut existing_by_question = existing_answers
        .into_iter()
        .map(|answer| (answer.question_id, answer))
        .collect::<HashMap<_, _>>();

    for (question_id, selected_option_id, answer_text, score) in validated_answers {
        let existing = existing_by_question.remove(&question_id);

        if selected_option_id.is_none() && answer_text.is_none() {
            if let Some(existing) = existing {
                let active: QuizAnswerActiveModel = existing.into();
                if let Err(err) = active.delete(&transaction).await {
                    let _ = transaction.rollback().await;
                    return HttpResponse::InternalServerError()
                        .body(format!("Answer save delete error: {}", err));
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
            return HttpResponse::InternalServerError().body(format!("Answer save error: {}", err));
        }
    }

    if let Err(err) = transaction.commit().await {
        return HttpResponse::InternalServerError()
            .body(format!("Answer save commit error: {}", err));
    }

    match QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .all(db)
        .await
    {
        Ok(answers) => HttpResponse::Ok().json(quiz_helper::saved_answer_payload(answers)),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn grade_answer(
    db: &DatabaseConnection,
    session: &Session,
    answer_id: i32,
    data: GradeQuizAnswer,
) -> HttpResponse {
    if let Err(response) = quiz_helper::require_staff(session) {
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

            if let Err(response) =
                quiz_helper::require_can_manage_quiz(db, session, question.quiz_id).await
            {
                return response;
            }

            let attempt_id = record.attempt_id;
            let transaction = match db.begin().await {
                Ok(transaction) => transaction,
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .body(format!("Could not start grading: {}", err));
                }
            };
            if let Err(response) = quiz_helper::lock_attempt(&transaction, attempt_id).await {
                let _ = transaction.rollback().await;
                return response;
            }
            let mut active: QuizAnswerActiveModel = record.into();
            active.score = Set(Some(data.score));
            active.feedback = Set(Some(data.feedback));

            if let Err(err) = active.update(&transaction).await {
                let _ = transaction.rollback().await;
                return HttpResponse::InternalServerError().body(format!("Update error: {}", err));
            }

            let answers = match QuizAnswerEntity::find()
                .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
                .all(&transaction)
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
                .all(&transaction)
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

            match QuizAttemptEntity::find_by_id(attempt_id)
                .one(&transaction)
                .await
            {
                Ok(Some(attempt)) => {
                    let mut active_attempt: QuizAttemptActiveModel = attempt.into();
                    active_attempt.total_score = Set(Some(total_score));
                    active_attempt.is_graded = Set(is_graded);

                    match active_attempt.update(&transaction).await {
                        Ok(_) => match transaction.commit().await {
                            Ok(()) => HttpResponse::Ok()
                                .body(format!("Answer {} graded successfully!", answer_id)),
                            Err(err) => HttpResponse::InternalServerError()
                                .body(format!("Grade commit error: {}", err)),
                        },
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
