use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set, TransactionTrait,
};
use serde::Serialize;
use std::collections::HashSet;

use crate::entity::quiz::{Column as QuizColumn, Entity as QuizEntity, Model as QuizModel};
use crate::entity::quiz_answers::{Column as QuizAnswerColumn, Entity as QuizAnswerEntity};
use crate::entity::quiz_attempts::{
    ActiveModel as QuizAttemptActiveModel, Column as QuizAttemptColumn, Entity as QuizAttemptEntity,
};
use crate::entity::quiz_questions::QuestionType;
use crate::models::quiz_attempts::{AttemptAccess, AttemptTimer, StartAttemptResponse};
use crate::services::auth_helpers::{get_role_ids, has_staff_role, is_student_only};
use crate::services::prerequisite_service;
use crate::services::quiz_helper::{self, QuizResult, QuizServiceError};

#[derive(Serialize)]
struct QuizAttemptStatus {
    quiz_id: i32,
    attempts_used: usize,
    attempts_left: Option<i32>,
    max_attempts: Option<i32>,
    has_submitted_attempt: bool,
    can_attempt: bool,
    message: String,
}

struct AttemptStatusContext {
    quiz_id: i32,
    attempts_used: usize,
    attempts_left: Option<i32>,
    max_attempts: Option<i32>,
    has_submitted_attempt: bool,
}

fn quiz_attempt_status(
    context: &AttemptStatusContext,
    can_attempt: bool,
    message: impl Into<String>,
) -> QuizAttemptStatus {
    QuizAttemptStatus {
        quiz_id: context.quiz_id,
        attempts_used: context.attempts_used,
        attempts_left: context.attempts_left,
        max_attempts: context.max_attempts,
        has_submitted_attempt: context.has_submitted_attempt,
        can_attempt,
        message: message.into(),
    }
}

fn attempt_status_context(
    quiz: &QuizModel,
    attempts: &[crate::entity::quiz_attempts::Model],
) -> AttemptStatusContext {
    AttemptStatusContext {
        quiz_id: quiz.quiz_id,
        attempts_used: attempts.len(),
        attempts_left: quiz
            .max_attempts
            .map(|max| (max - attempts.len() as i32).max(0)),
        max_attempts: quiz.max_attempts,
        has_submitted_attempt: attempts
            .iter()
            .any(|attempt| attempt.submitted_at.is_some()),
    }
}

fn build_attempt_timer(
    quiz: &QuizModel,
    attempt: Option<&crate::entity::quiz_attempts::Model>,
) -> AttemptTimer {
    let expires_at = attempt
        .and_then(|attempt| quiz_helper::attempt_expires_at(quiz.time_limit, attempt.started_at))
        .map(|expires_at| expires_at.format("%Y-%m-%dT%H:%M:%S").to_string());

    AttemptTimer {
        time_limit_minutes: quiz.time_limit,
        expires_at,
        remaining_seconds: attempt.and_then(|attempt| {
            quiz_helper::attempt_expires_at(quiz.time_limit, attempt.started_at)
                .map(|expires_at| (expires_at - quiz_helper::quiz_now()).num_seconds().max(0))
        }),
        message: quiz
            .time_limit
            .map(|minutes| format!("{} minute time limit", minutes))
            .unwrap_or_else(|| "No time limit".to_string()),
    }
}

async fn build_start_response(
    db: &impl ConnectionTrait,
    quiz: QuizModel,
    attempt: Option<crate::entity::quiz_attempts::Model>,
    access: AttemptAccess,
) -> QuizResult<StartAttemptResponse> {
    let questions = quiz_helper::load_attempt_questions(db, quiz.quiz_id).await?;
    let answers = if let Some(attempt) = attempt.as_ref() {
        quiz_helper::saved_answer_payload(
            quiz_helper::load_answers_for_attempt(db, attempt.attempt_id).await?,
        )
    } else {
        Vec::new()
    };
    Ok(StartAttemptResponse {
        timer: build_attempt_timer(&quiz, attempt.as_ref()),
        quiz,
        questions,
        access,
        attempt,
        answers,
    })
}

async fn finalize_attempt(
    db: &impl ConnectionTrait,
    attempt: crate::entity::quiz_attempts::Model,
) -> QuizResult<()> {
    let answers = quiz_helper::load_answers_for_attempt(db, attempt.attempt_id).await?;

    let short_answer_question_ids = quiz_helper::load_quiz_questions(db, attempt.quiz_id)
        .await?
        .into_iter()
        .filter(|question| question.question_type == QuestionType::LongAnswer)
        .map(|question| question.question_id)
        .collect::<HashSet<_>>();
    let requires_manual_grading = answers
        .iter()
        .any(|answer| short_answer_question_ids.contains(&answer.question_id));

    let total_score = answers
        .iter()
        .filter_map(|answer| answer.score)
        .sum::<i32>();
    let mut active: QuizAttemptActiveModel = attempt.into();
    active.submitted_at = Set(Some(quiz_helper::quiz_now()));
    active.total_score = Set(Some(total_score));
    active.is_graded = Set(!requires_manual_grading);

    active
        .update(db)
        .await
        .map(|_| ())
        .map_err(|err| quiz_helper::internal_service_error(format!("Update error: {}", err)))
}
fn build_attempt_status(
    quiz: &QuizModel,
    attempts: &[crate::entity::quiz_attempts::Model],
) -> QuizAttemptStatus {
    let context = attempt_status_context(&quiz, attempts);

    if let Some(starts_at) = quiz.starts_at {
        if starts_at > quiz_helper::quiz_now() {
            return quiz_attempt_status(&context, false, "This quiz is not open yet");
        }
    }

    if let Some(ends_at) = quiz.ends_at {
        if ends_at < quiz_helper::quiz_now() {
            return quiz_attempt_status(&context, false, "This quiz is closed");
        }
    }

    if context.attempts_left == Some(0) {
        return quiz_attempt_status(&context, false, "No attempts left");
    }

    let message = match context.attempts_left {
        Some(1) => "1 attempt left".to_string(),
        Some(left) => format!("{} attempts left", left),
        None if context.has_submitted_attempt => "Attempted".to_string(),
        None => "Unlimited attempts".to_string(),
    };

    quiz_attempt_status(&context, true, message)
}

async fn get_user_attempts_for_quiz(
    db: &impl ConnectionTrait,
    quiz_id: i32,
    user_id: i32,
) -> QuizResult<Vec<crate::entity::quiz_attempts::Model>> {
    QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::QuizId.eq(quiz_id))
        .filter(QuizAttemptColumn::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(quiz_helper::db_service_error)
}

async fn build_attempt_status_for_quiz(
    db: &DatabaseConnection,
    user_id: i32,
    quiz: &QuizModel,
) -> QuizResult<QuizAttemptStatus> {
    let attempts = get_user_attempts_for_quiz(db, quiz.quiz_id, user_id).await?;

    if let Some(prerequisite) =
        prerequisite_service::get_first_incomplete_required_module_for_service(
            db,
            user_id,
            prerequisite_service::get_quiz_prerequisite_ids_for_service(db, quiz.quiz_id).await?,
        )
        .await?
    {
        return Ok(quiz_attempt_status(
            &attempt_status_context(quiz, &attempts),
            false,
            format!(
                "Complete {} before attempting this quiz",
                prerequisite.title
            ),
        ));
    }

    Ok(build_attempt_status(quiz, &attempts))
}

fn staff_preview_access() -> AttemptAccess {
    AttemptAccess {
        can_attempt: false,
        preview_only: true,
        message: "You can view the questions, but instructors and admins cannot attempt quizzes."
            .to_string(),
    }
}

fn student_attempt_access() -> AttemptAccess {
    AttemptAccess {
        can_attempt: true,
        preview_only: false,
        message: "You can attempt this quiz.".to_string(),
    }
}

async fn load_quiz(db: &impl ConnectionTrait, quiz_id: i32) -> QuizResult<QuizModel> {
    QuizEntity::find_by_id(quiz_id)
        .one(db)
        .await
        .map_err(quiz_helper::db_service_error)?
        .ok_or_else(|| QuizServiceError::NotFound("Quiz not found".to_string()))
}

async fn ensure_prerequisites_complete(
    db: &impl ConnectionTrait,
    user_id: i32,
    quiz: &QuizModel,
) -> QuizResult<()> {
    let prerequisite_ids =
        prerequisite_service::get_quiz_prerequisite_ids_for_service(db, quiz.quiz_id).await?;

    match prerequisite_service::get_first_incomplete_required_module_for_service(
        db,
        user_id,
        prerequisite_ids,
    )
    .await?
    {
        Some(prerequisite) => Err(QuizServiceError::Forbidden(format!(
            "Complete {} before attempting this quiz",
            prerequisite.title
        ))),
        None => Ok(()),
    }
}

async fn ensure_student_enrolled_for_quiz(
    db: &DatabaseConnection,
    user_id: i32,
    quiz: &QuizModel,
) -> QuizResult<()> {
    match quiz_helper::is_enrolled_for_service(db, user_id, quiz.course_id).await {
        Ok(true) => {}
        Ok(false) => {
            return Err(QuizServiceError::Forbidden(
                "You must be enrolled to attempt this quiz".to_string(),
            ));
        }
        Err(error) => return Err(error),
    }

    Ok(())
}

async fn build_staff_preview_response(
    db: &DatabaseConnection,
    session: &Session,
    quiz: QuizModel,
) -> QuizResult<StartAttemptResponse> {
    quiz_helper::require_can_manage_quiz(db, session, quiz.quiz_id).await?;
    build_start_response(db, quiz, None, staff_preview_access()).await
}

async fn resume_open_attempt_or_finalize_expired(
    db: &impl ConnectionTrait,
    quiz: QuizModel,
    open_attempt: crate::entity::quiz_attempts::Model,
) -> QuizResult<Option<StartAttemptResponse>> {
    if !quiz_helper::attempt_time_limit_expired(quiz.time_limit, open_attempt.started_at) {
        return build_start_response(db, quiz, Some(open_attempt), student_attempt_access())
            .await
            .map(Some);
    }

    quiz_helper::lock_attempt_for_service(db, open_attempt.attempt_id).await?;
    finalize_attempt(db, open_attempt).await?;
    Ok(None)
}

async fn start_student_attempt_in_transaction(
    db: &impl ConnectionTrait,
    quiz_id: i32,
    user_id: i32,
) -> QuizResult<StartAttemptResponse> {
    quiz_helper::lock_quiz_for_service(db, quiz_id).await?;

    let quiz = load_quiz(db, quiz_id).await?;
    ensure_prerequisites_complete(db, user_id, &quiz).await?;

    let mut attempts = get_user_attempts_for_quiz(db, quiz_id, user_id).await?;
    if let Some(open_attempt) = attempts
        .iter()
        .find(|attempt| attempt.submitted_at.is_none())
        .cloned()
    {
        if let Some(response) =
            resume_open_attempt_or_finalize_expired(db, quiz.clone(), open_attempt).await?
        {
            return Ok(response);
        }
        attempts = get_user_attempts_for_quiz(db, quiz_id, user_id).await?;
    }

    let status = build_attempt_status(&quiz, &attempts);

    if !status.can_attempt {
        return Err(QuizServiceError::Forbidden(status.message));
    }

    if let Some(max_attempts) = quiz.max_attempts {
        if attempts.len() >= max_attempts as usize {
            return Err(QuizServiceError::Forbidden(
                "Maximum quiz attempts reached".to_string(),
            ));
        }
    }

    let attempt = QuizAttemptActiveModel {
        quiz_id: Set(quiz_id),
        user_id: Set(user_id),
        started_at: Set(quiz_helper::quiz_now()),
        is_graded: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|err| quiz_helper::internal_service_error(format!("Insert error: {}", err)))?;

    build_start_response(db, quiz, Some(attempt), student_attempt_access()).await
}

async fn create_attempt_response(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> QuizResult<StartAttemptResponse> {
    let user_id = quiz_helper::get_user_id_for_service(session)?;
    let role_ids = get_role_ids(session);
    let quiz = load_quiz(db, quiz_id).await?;

    if has_staff_role(&role_ids) {
        return build_staff_preview_response(db, session, quiz).await;
    }

    if !is_student_only(&role_ids) {
        return Err(QuizServiceError::Forbidden(
            "Student role required to attempt this quiz".to_string(),
        ));
    }

    ensure_student_enrolled_for_quiz(db, user_id, &quiz).await?;

    let transaction = db.begin().await.map_err(|err| {
        quiz_helper::internal_service_error(format!("Could not start attempt: {}", err))
    })?;
    let response = match start_student_attempt_in_transaction(&transaction, quiz_id, user_id).await
    {
        Ok(response) => response,
        Err(response) => {
            let _ = transaction.rollback().await;
            return Err(response);
        }
    };

    transaction.commit().await.map_err(|err| {
        quiz_helper::internal_service_error(format!("Attempt transaction error: {}", err))
    })?;

    Ok(response)
}

pub async fn list_my_attempt_statuses_by_course(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> HttpResponse {
    let user_id = match quiz_helper::get_user_id_for_service(session) {
        Ok(id) => id,
        Err(error) => return error.into_response(),
    };

    let quizzes = match QuizEntity::find()
        .filter(QuizColumn::CourseId.eq(course_id))
        .all(db)
        .await
    {
        Ok(quizzes) => quizzes,
        Err(err) => return quiz_helper::db_service_error(err).into_response(),
    };

    let mut statuses = Vec::with_capacity(quizzes.len());

    for quiz in quizzes {
        match build_attempt_status_for_quiz(db, user_id, &quiz).await {
            Ok(status) => statuses.push(status),
            Err(error) => return error.into_response(),
        }
    }

    HttpResponse::Ok().json(statuses)
}

pub async fn create_attempt(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> HttpResponse {
    match create_attempt_response(db, session, quiz_id).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => error.into_response(),
    }
}

async fn submit_attempt_in_transaction(
    db: &impl ConnectionTrait,
    user_id: i32,
    attempt_id: i32,
) -> QuizResult<()> {
    quiz_helper::lock_attempt_for_service(db, attempt_id).await?;

    let attempt = QuizAttemptEntity::find_by_id(attempt_id)
        .one(db)
        .await
        .map_err(quiz_helper::db_service_error)?
        .ok_or_else(|| QuizServiceError::NotFound("Attempt not found".to_string()))?;

    if attempt.user_id != user_id {
        return Err(QuizServiceError::Forbidden(
            "You can only submit your own attempt".to_string(),
        ));
    }

    if attempt.submitted_at.is_some() {
        return Err(QuizServiceError::BadRequest(
            "This attempt has already been submitted".to_string(),
        ));
    }

    let quiz = load_quiz(db, attempt.quiz_id).await?;
    let time_limit_expired =
        quiz_helper::attempt_time_limit_expired(quiz.time_limit, attempt.started_at);

    let answers = quiz_helper::load_answers_for_attempt(db, attempt_id).await?;
    let questions = quiz_helper::load_quiz_questions(db, attempt.quiz_id).await?;
    let answered_question_ids = answers
        .iter()
        .map(|answer| answer.question_id)
        .collect::<HashSet<i32>>();

    if !time_limit_expired
        && questions
            .iter()
            .any(|question| !answered_question_ids.contains(&question.question_id))
    {
        return Err(QuizServiceError::BadRequest(
            "All questions must be answered before submission".to_string(),
        ));
    }

    finalize_attempt(db, attempt).await
}

pub async fn submit_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
) -> HttpResponse {
    let user_id = match quiz_helper::get_user_id_for_service(session) {
        Ok(id) => id,
        Err(error) => return error.into_response(),
    };
    let role_ids = get_role_ids(session);

    if has_staff_role(&role_ids) {
        return HttpResponse::Forbidden()
            .body("Instructors and admins can view quiz questions but cannot submit attempts");
    }

    if !is_student_only(&role_ids) {
        return HttpResponse::Forbidden().body("Student role required to submit this quiz");
    }

    let transaction = match db.begin().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Could not start submission: {}", err));
        }
    };

    if let Err(response) = submit_attempt_in_transaction(&transaction, user_id, attempt_id).await {
        let _ = transaction.rollback().await;
        return response.into_response();
    }

    match transaction.commit().await {
        Ok(_) => HttpResponse::Ok().body(format!("Attempt {} submitted", attempt_id)),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Submission transaction error: {}", err)),
    }
}

async fn delete_attempt_in_transaction(
    db: &impl ConnectionTrait,
    attempt_id: i32,
) -> QuizResult<()> {
    quiz_helper::lock_attempt_for_service(db, attempt_id).await?;

    QuizAnswerEntity::delete_many()
        .filter(QuizAnswerColumn::AttemptId.eq(attempt_id))
        .exec(db)
        .await
        .map_err(|err| {
            quiz_helper::internal_service_error(format!("Attempt answer delete error: {}", err))
        })?;

    match QuizAttemptEntity::delete_by_id(attempt_id).exec(db).await {
        Ok(result) if result.rows_affected > 0 => Ok(()),
        Ok(_) => Err(QuizServiceError::NotFound("Attempt not found".to_string())),
        Err(err) => Err(quiz_helper::internal_service_error(format!(
            "Attempt delete error: {}",
            err
        ))),
    }
}

pub async fn delete_attempt(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
) -> HttpResponse {
    if let Err(response) = quiz_helper::require_staff(session) {
        return response.into_response();
    }

    let attempt = match QuizAttemptEntity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => attempt,
        Ok(None) => return HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => return quiz_helper::db_service_error(err).into_response(),
    };

    if let Err(response) = quiz_helper::require_can_manage_quiz(db, session, attempt.quiz_id).await
    {
        return response.into_response();
    }

    let transaction = match db.begin().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Could not start attempt delete: {}", err));
        }
    };

    if let Err(response) = delete_attempt_in_transaction(&transaction, attempt_id).await {
        let _ = transaction.rollback().await;
        return response.into_response();
    }

    match transaction.commit().await {
        Ok(_) => HttpResponse::Ok().body("Quiz attempt deleted"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Attempt delete commit error: {}", err)),
    }
}
