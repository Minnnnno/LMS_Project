use actix_session::Session;
use actix_web::HttpResponse;
use chrono::Duration;
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
use crate::services::auth_helpers::{get_role_ids, is_student_only};
use crate::services::quiz_helper::{self, QuizResult, QuizServiceError};

const AUTO_SUBMIT_GRACE_SECONDS: i64 = 30;
type ValidatedQuizAnswer = (i32, Option<i32>, Option<String>, Option<i32>);

async fn load_accessible_attempt_result(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
    forbidden_message: &str,
) -> QuizResult<QuizAttemptModel> {
    let user_id = quiz_helper::get_user_id_for_service(session)?;
    let role_ids = get_role_ids(session);

    match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => {
            if is_student_only(&role_ids) && attempt.user_id != user_id {
                Err(QuizServiceError::Forbidden(forbidden_message.to_string()))
            } else {
                Ok(attempt)
            }
        }
        Ok(None) => Err(QuizServiceError::NotFound("Attempt not found".to_string())),
        Err(err) => Err(quiz_helper::db_service_error(err)),
    }
}

async fn ensure_attempt_accepts_answers_result(
    db: &impl sea_orm::ConnectionTrait,
    attempt: &QuizAttemptModel,
) -> QuizResult<()> {
    if attempt.submitted_at.is_some() {
        return Err(QuizServiceError::BadRequest(
            "This attempt has already been submitted".to_string(),
        ));
    }

    let quiz = QuizEntity::find_by_id(attempt.quiz_id)
        .one(db)
        .await
        .map_err(quiz_helper::db_service_error)?
        .ok_or_else(|| QuizServiceError::NotFound("Quiz not found".to_string()))?;

    if let Some(minutes) = quiz.time_limit {
        let expires_at = attempt.started_at + Duration::minutes(minutes as i64);
        let cutoff = expires_at + Duration::seconds(AUTO_SUBMIT_GRACE_SECONDS);

        if quiz_helper::quiz_now() > cutoff {
            return Err(QuizServiceError::BadRequest(
                "The time limit for this quiz has ended".to_string(),
            ));
        }
    }

    Ok(())
}

fn validate_submitted_answers_result(
    data: SaveQuizAnswers,
    questions_by_id: &HashMap<i32, crate::entity::quiz_questions::Model>,
    options_by_id: &HashMap<i32, crate::entity::quiz_options::Model>,
) -> QuizResult<Vec<ValidatedQuizAnswer>> {
    let mut submitted_question_ids = HashSet::new();
    let mut validated_answers = Vec::with_capacity(data.answers.len());

    for answer in data.answers {
        if !submitted_question_ids.insert(answer.question_id) {
            return Err(QuizServiceError::BadRequest(
                "Each question may only appear once".to_string(),
            ));
        }

        let question = questions_by_id.get(&answer.question_id).ok_or_else(|| {
            QuizServiceError::BadRequest(
                "Question does not belong to this attempt's quiz".to_string(),
            )
        })?;

        match question.question_type {
            QuestionType::Mcq => {
                if answer
                    .answer_text
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty())
                {
                    return Err(QuizServiceError::BadRequest(
                        "MCQ answers cannot contain answer text".to_string(),
                    ));
                }

                let option = match answer.selected_option_id {
                    Some(option_id) => match options_by_id.get(&option_id) {
                        Some(option) if option.question_id == answer.question_id => Some(option),
                        _ => {
                            return Err(QuizServiceError::BadRequest(
                                "Selected option does not belong to this question".to_string(),
                            ));
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
                    return Err(QuizServiceError::BadRequest(
                        "Long answers cannot contain a selected option".to_string(),
                    ));
                }

                let answer_text = answer.answer_text.filter(|text| !text.trim().is_empty());
                validated_answers.push((answer.question_id, None, answer_text, None));
            }
        }
    }

    if submitted_question_ids.len() != questions_by_id.len() {
        return Err(QuizServiceError::BadRequest(
            "Autosave must include every quiz question".to_string(),
        ));
    }

    Ok(validated_answers)
}

pub async fn save_answers(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
    data: SaveQuizAnswers,
) -> HttpResponse {
    if let Err(response) = quiz_helper::require_student(session) {
        return response.into_response();
    }

    match save_answers_result(db, session, attempt_id, data).await {
        Ok(answers) => HttpResponse::Ok().json(quiz_helper::saved_answer_payload(answers)),
        Err(error) => error.into_response(),
    }
}

async fn save_answers_result(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
    data: SaveQuizAnswers,
) -> QuizResult<Vec<crate::entity::quiz_answers::Model>> {
    let attempt = load_accessible_attempt_result(
        db,
        session,
        attempt_id,
        "You can only save answers for your own attempts",
    )
    .await?;

    ensure_attempt_accepts_answers_result(db, &attempt).await?;

    let questions = quiz_helper::load_quiz_questions(db, attempt.quiz_id).await?;
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
            Err(err) => return Err(quiz_helper::db_service_error(err)),
        }
    };
    let validated_answers =
        validate_submitted_answers_result(data, &questions_by_id, &options_by_id)?;

    let transaction = match db.begin().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return Err(QuizServiceError::Internal(format!(
                "Could not start answer save: {}",
                err
            )));
        }
    };

    if let Err(error) = quiz_helper::lock_attempt_for_service(&transaction, attempt_id).await {
        let _ = transaction.rollback().await;
        return Err(error);
    }

    let locked_attempt = match QuizAttemptEntity::find_by_id(attempt_id)
        .one(&transaction)
        .await
    {
        Ok(Some(attempt)) => attempt,
        Ok(None) => {
            let _ = transaction.rollback().await;
            return Err(QuizServiceError::NotFound("Attempt not found".to_string()));
        }
        Err(err) => {
            let _ = transaction.rollback().await;
            return Err(quiz_helper::db_service_error(err));
        }
    };
    if let Err(error) = ensure_attempt_accepts_answers_result(&transaction, &locked_attempt).await {
        let _ = transaction.rollback().await;
        return Err(error);
    }

    let existing_answers =
        match quiz_helper::load_answers_for_attempt(&transaction, attempt_id).await {
            Ok(answers) => answers,
            Err(error) => {
                let _ = transaction.rollback().await;
                return Err(error);
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
                    return Err(QuizServiceError::Internal(format!(
                        "Answer save delete error: {}",
                        err
                    )));
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
            return Err(QuizServiceError::Internal(format!(
                "Answer save error: {}",
                err
            )));
        }
    }

    if let Err(err) = transaction.commit().await {
        return Err(QuizServiceError::Internal(format!(
            "Answer save commit error: {}",
            err
        )));
    }

    QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .all(db)
        .await
        .map_err(quiz_helper::db_service_error)
}

pub async fn grade_answer(
    db: &DatabaseConnection,
    session: &Session,
    answer_id: i32,
    data: GradeQuizAnswer,
) -> HttpResponse {
    if let Err(response) = quiz_helper::require_staff(session) {
        return response.into_response();
    }

    match grade_answer_result(db, session, answer_id, data).await {
        Ok(message) => HttpResponse::Ok().body(message),
        Err(error) => error.into_response(),
    }
}

async fn grade_answer_result(
    db: &DatabaseConnection,
    session: &Session,
    answer_id: i32,
    data: GradeQuizAnswer,
) -> QuizResult<String> {
    let record = QuizAnswerEntity::find_by_id(answer_id)
        .one(db)
        .await
        .map_err(quiz_helper::db_service_error)?
        .ok_or_else(|| QuizServiceError::NotFound("Answer not found".to_string()))?;

    let question = QuizQuestionEntity::find_by_id(record.question_id)
        .one(db)
        .await
        .map_err(quiz_helper::db_service_error)?
        .ok_or_else(|| QuizServiceError::NotFound("Question not found".to_string()))?;

    validate_manual_grade(&question, &data)?;

    quiz_helper::require_can_manage_quiz(db, session, question.quiz_id).await?;

    apply_manual_grade(db, record, question.quiz_id, data).await?;

    Ok(format!("Answer {} graded successfully!", answer_id))
}

fn validate_manual_grade(
    question: &crate::entity::quiz_questions::Model,
    data: &GradeQuizAnswer,
) -> QuizResult<()> {
    if question.question_type != QuestionType::LongAnswer {
        return Err(QuizServiceError::BadRequest(
            "Only long answer questions can be manually graded".to_string(),
        ));
    }

    if data.score < 0 {
        return Err(QuizServiceError::BadRequest(
            "Score must be 0 or higher".to_string(),
        ));
    }

    if data.score > question.points {
        return Err(QuizServiceError::BadRequest(format!(
            "Marks awarded exceed how much this question is worth ({})",
            question.points
        )));
    }

    Ok(())
}

async fn apply_manual_grade(
    db: &DatabaseConnection,
    record: crate::entity::quiz_answers::Model,
    quiz_id: i32,
    data: GradeQuizAnswer,
) -> QuizResult<()> {
    let attempt_id = record.attempt_id;
    let transaction = db
        .begin()
        .await
        .map_err(|err| QuizServiceError::Internal(format!("Could not start grading: {}", err)))?;

    if let Err(error) = quiz_helper::lock_attempt_for_service(&transaction, attempt_id).await {
        let _ = transaction.rollback().await;
        return Err(error);
    }

    let mut active: QuizAnswerActiveModel = record.into();
    active.score = Set(Some(data.score));
    active.feedback = Set(Some(data.feedback));

    if let Err(err) = active.update(&transaction).await {
        let _ = transaction.rollback().await;
        return Err(QuizServiceError::Internal(format!("Update error: {}", err)));
    }

    let answers = match QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .all(&transaction)
        .await
    {
        Ok(answers) => answers,
        Err(err) => {
            let _ = transaction.rollback().await;
            return Err(quiz_helper::db_service_error(err));
        }
    };

    let total_score = answers
        .iter()
        .filter_map(|answer| answer.score)
        .sum::<i32>();

    let long_answer_questions = match QuizQuestionEntity::find()
        .filter(QuizQuestionColumn::QuizId.eq(quiz_id))
        .filter(QuizQuestionColumn::QuestionType.eq(QuestionType::LongAnswer))
        .all(&transaction)
        .await
    {
        Ok(questions) => questions,
        Err(err) => {
            let _ = transaction.rollback().await;
            return Err(quiz_helper::db_service_error(err));
        }
    };

    let is_graded = long_answer_questions.iter().all(|question| {
        answers
            .iter()
            .find(|answer| answer.question_id == question.question_id)
            .map(|answer| answer.score.is_some())
            .unwrap_or(true)
    });

    let attempt = match QuizAttemptEntity::find_by_id(attempt_id)
        .one(&transaction)
        .await
    {
        Ok(Some(attempt)) => attempt,
        Ok(None) => {
            let _ = transaction.rollback().await;
            return Err(QuizServiceError::NotFound("Attempt not found".to_string()));
        }
        Err(err) => {
            let _ = transaction.rollback().await;
            return Err(quiz_helper::db_service_error(err));
        }
    };

    let mut active_attempt: QuizAttemptActiveModel = attempt.into();
    active_attempt.total_score = Set(Some(total_score));
    active_attempt.is_graded = Set(is_graded);

    if let Err(err) = active_attempt.update(&transaction).await {
        let _ = transaction.rollback().await;
        return Err(QuizServiceError::Internal(format!(
            "Attempt score update error: {}",
            err
        )));
    }

    transaction
        .commit()
        .await
        .map_err(|err| QuizServiceError::Internal(format!("Grade commit error: {}", err)))?;

    Ok(())
}
