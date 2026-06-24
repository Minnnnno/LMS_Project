use std::collections::HashMap;

use actix_session::Session;
use actix_web::HttpResponse;
use chrono::{Duration, FixedOffset, NaiveDateTime, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait, QueryFilter,
    QueryOrder, Statement,
};

use crate::entity::{
    course_instructors, courses, enrollments, quiz, quiz_answers, quiz_attempts, quiz_options,
    quiz_questions, users,
};
use crate::models::quiz::{SaveQuizOption, SaveQuizQuestion};
use crate::models::quiz_answers::SavedQuizAnswer;
use crate::models::quiz_attempts::{AttemptQuizOption, AttemptQuizQuestion};
use crate::services::auth_helpers::{get_role_ids, has_staff_role, is_student_only};
use crate::services::course_service::has_role;

pub type QuizResult<T> = Result<T, QuizServiceError>;

pub enum QuizServiceError {
    NotFound(String),
    Forbidden(String),
    BadRequest(String),
    Conflict(String),
    Database(String),
    Internal(String),
    Unauthorized(String),
}

impl QuizServiceError {
    pub fn into_response(self) -> HttpResponse {
        match self {
            Self::NotFound(message) => HttpResponse::NotFound().body(message),
            Self::Forbidden(message) => HttpResponse::Forbidden().body(message),
            Self::BadRequest(message) => HttpResponse::BadRequest().body(message),
            Self::Conflict(message) => HttpResponse::Conflict().body(message),
            Self::Database(message) | Self::Internal(message) => {
                HttpResponse::InternalServerError().body(message)
            }
            Self::Unauthorized(message) => HttpResponse::Unauthorized().body(message),
        }
    }
}

impl From<QuizServiceError> for HttpResponse {
    fn from(error: QuizServiceError) -> Self {
        error.into_response()
    }
}

pub fn db_service_error(err: DbErr) -> QuizServiceError {
    QuizServiceError::Database(format!("Database error: {}", err))
}

pub fn internal_service_error(message: impl Into<String>) -> QuizServiceError {
    QuizServiceError::Internal(message.into())
}

pub fn quiz_now() -> NaiveDateTime {
    let singapore_offset =
        FixedOffset::east_opt(8 * 60 * 60).expect("Singapore UTC offset must be valid");

    Utc::now().with_timezone(&singapore_offset).naive_local()
}

pub fn get_user_id_for_service(session: &Session) -> QuizResult<i32> {
    match session.get::<i32>("user_id") {
        Ok(Some(id)) => Ok(id),
        Ok(None) => Err(QuizServiceError::Unauthorized(
            "You must be logged in".to_string(),
        )),
        Err(_) => Err(QuizServiceError::Internal("Session error".to_string())),
    }
}

pub async fn is_enrolled_for_service(
    db: &DatabaseConnection,
    user_id: i32,
    course_id: i32,
) -> QuizResult<bool> {
    enrollments::Entity::find()
        .filter(enrollments::Column::UserId.eq(user_id))
        .filter(enrollments::Column::CourseId.eq(course_id))
        .one(db)
        .await
        .map(|enrollment| enrollment.is_some())
        .map_err(|err| {
            QuizServiceError::Internal(format!("Database error checking enrollment: {}", err))
        })
}

pub fn require_staff(session: &Session) -> QuizResult<()> {
    let role_ids = get_role_ids(session);
    if role_ids.is_empty() {
        return Err(QuizServiceError::Unauthorized(
            "You must be logged in".to_string(),
        ));
    }
    if !has_staff_role(&role_ids) {
        return Err(QuizServiceError::Forbidden(
            "Staff role required".to_string(),
        ));
    }
    Ok(())
}

pub fn require_student(session: &Session) -> QuizResult<()> {
    let role_ids = get_role_ids(session);
    if !is_student_only(&role_ids) {
        return Err(QuizServiceError::Forbidden(
            "Student role required".to_string(),
        ));
    }
    Ok(())
}

pub async fn require_can_manage_course_id(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> QuizResult<courses::Model> {
    let course = courses::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(db_service_error)?
        .ok_or_else(|| QuizServiceError::NotFound("Course not found".to_string()))?;

    if can_manage_course_for_service(db, session, &course).await? {
        Ok(course)
    } else {
        Err(QuizServiceError::Forbidden(
            "You cannot manage quizzes for this course".to_string(),
        ))
    }
}

pub async fn require_can_manage_quiz(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> QuizResult<quiz::Model> {
    let quiz = quiz::Entity::find_by_id(quiz_id)
        .one(db)
        .await
        .map_err(db_service_error)?
        .ok_or_else(|| QuizServiceError::NotFound("Quiz not found".to_string()))?;

    require_can_manage_course_id(db, session, quiz.course_id).await?;
    Ok(quiz)
}

pub async fn ensure_content_editable(db: &impl ConnectionTrait, quiz_id: i32) -> QuizResult<()> {
    let has_attempt = quiz_attempts::Entity::find()
        .filter(quiz_attempts::Column::QuizId.eq(quiz_id))
        .one(db)
        .await
        .map_err(db_service_error)?
        .is_some();

    if has_attempt {
        return Err(QuizServiceError::Conflict(
            "Quiz content cannot be changed after attempts have started".to_string(),
        ));
    }
    Ok(())
}

pub async fn can_manage_course_for_service(
    db: &DatabaseConnection,
    session: &Session,
    course: &courses::Model,
) -> QuizResult<bool> {
    if has_role(session, "LMS Admin") {
        return Ok(true);
    }

    let user_id = get_user_id_for_service(session)?;
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|err| QuizServiceError::Internal(format!("Database error finding user: {}", err)))?
        .ok_or_else(|| QuizServiceError::NotFound("User not found".to_string()))?;

    if has_role(session, "Organisation Admin")
        && user.org_id.is_some()
        && user.org_id == course.org_id
    {
        return Ok(true);
    }

    if has_role(session, "Instructor") {
        return course_instructors::Entity::find_by_id((course.course_id, user.user_id))
            .one(db)
            .await
            .map(|assignment| assignment.is_some())
            .map_err(|err| {
                QuizServiceError::Internal(format!(
                    "Database error finding course instructor: {}",
                    err
                ))
            });
    }

    Ok(false)
}

pub async fn lock_quiz_for_service(db: &impl ConnectionTrait, quiz_id: i32) -> QuizResult<()> {
    advisory_lock(db, 2, quiz_id, "Quiz content").await
}

pub async fn lock_attempt_for_service(
    db: &impl ConnectionTrait,
    attempt_id: i32,
) -> QuizResult<()> {
    advisory_lock(db, 1, attempt_id, "Attempt").await
}

async fn advisory_lock(
    db: &impl ConnectionTrait,
    namespace: i32,
    id: i32,
    label: &str,
) -> QuizResult<()> {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1, $2)",
        [namespace.into(), id.into()],
    ))
    .await
    .map(|_| ())
    .map_err(|err| QuizServiceError::Internal(format!("{} lock error: {}", label, err)))
}

pub fn attempt_expires_at(
    time_limit_minutes: Option<i32>,
    started_at: NaiveDateTime,
) -> Option<NaiveDateTime> {
    time_limit_minutes.map(|minutes| started_at + Duration::minutes(minutes as i64))
}

pub fn attempt_time_limit_expired(
    time_limit_minutes: Option<i32>,
    started_at: NaiveDateTime,
) -> bool {
    attempt_expires_at(time_limit_minutes, started_at)
        .map(|expires_at| quiz_now() >= expires_at)
        .unwrap_or(false)
}

pub fn saved_answer_payload(answers: Vec<quiz_answers::Model>) -> Vec<SavedQuizAnswer> {
    answers
        .into_iter()
        .map(|answer| SavedQuizAnswer {
            question_id: answer.question_id,
            selected_option_id: answer.selected_option_id,
            answer_text: answer.answer_text,
        })
        .collect()
}

pub async fn load_quiz_questions(
    db: &impl ConnectionTrait,
    quiz_id: i32,
) -> QuizResult<Vec<quiz_questions::Model>> {
    quiz_questions::Entity::find()
        .filter(quiz_questions::Column::QuizId.eq(quiz_id))
        .order_by_asc(quiz_questions::Column::Position)
        .all(db)
        .await
        .map_err(db_service_error)
}

pub async fn load_options_for_questions(
    db: &impl ConnectionTrait,
    question_ids: Vec<i32>,
) -> QuizResult<Vec<quiz_options::Model>> {
    if question_ids.is_empty() {
        return Ok(Vec::new());
    }

    quiz_options::Entity::find()
        .filter(quiz_options::Column::QuestionId.is_in(question_ids))
        .order_by_asc(quiz_options::Column::QuestionId)
        .order_by_asc(quiz_options::Column::Position)
        .all(db)
        .await
        .map_err(db_service_error)
}

pub async fn load_options_for_quiz(
    db: &impl ConnectionTrait,
    questions: &[quiz_questions::Model],
) -> QuizResult<Vec<quiz_options::Model>> {
    let question_ids = questions
        .iter()
        .map(|question| question.question_id)
        .collect::<Vec<_>>();
    load_options_for_questions(db, question_ids).await
}

pub fn group_options_by_question(
    options: Vec<quiz_options::Model>,
) -> HashMap<i32, Vec<quiz_options::Model>> {
    let mut options_by_question = HashMap::<i32, Vec<quiz_options::Model>>::new();
    for option in options {
        options_by_question
            .entry(option.question_id)
            .or_default()
            .push(option);
    }
    options_by_question
}

pub async fn load_answers_for_attempt(
    db: &impl ConnectionTrait,
    attempt_id: i32,
) -> QuizResult<Vec<quiz_answers::Model>> {
    quiz_answers::Entity::find()
        .filter(quiz_answers::Column::AttemptId.eq(attempt_id))
        .all(db)
        .await
        .map_err(db_service_error)
}

pub async fn load_editor_questions(
    db: &impl ConnectionTrait,
    quiz_id: i32,
) -> QuizResult<Vec<SaveQuizQuestion>> {
    let questions = load_quiz_questions(db, quiz_id).await?;
    let mut options_by_question =
        group_options_by_question(load_options_for_quiz(db, &questions).await?);

    Ok(questions
        .into_iter()
        .map(|question| SaveQuizQuestion {
            question_type: question.question_type,
            question_text: question.question_text,
            position: question.position,
            points: question.points,
            options: options_by_question
                .remove(&question.question_id)
                .unwrap_or_default()
                .into_iter()
                .map(|option| SaveQuizOption {
                    option_text: option.option_text,
                    is_correct: option.is_correct,
                    position: option.position,
                })
                .collect(),
        })
        .collect())
}

pub async fn load_attempt_questions(
    db: &impl ConnectionTrait,
    quiz_id: i32,
) -> QuizResult<Vec<AttemptQuizQuestion>> {
    let questions = load_quiz_questions(db, quiz_id).await?;
    if questions.is_empty() {
        return Err(QuizServiceError::Conflict(
            "This quiz has no questions yet".to_string(),
        ));
    }
    let mut options_by_question =
        group_options_by_question(load_options_for_quiz(db, &questions).await?);

    questions
        .into_iter()
        .map(|question| {
            let option_rows = options_by_question
                .remove(&question.question_id)
                .unwrap_or_default();
            if question.question_type == quiz_questions::QuestionType::Mcq
                && (option_rows.len() < 2 || !option_rows.iter().any(|option| option.is_correct))
            {
                return Err(QuizServiceError::Conflict(
                    "This quiz has an invalid MCQ question and cannot be attempted yet".to_string(),
                ));
            }
            Ok(AttemptQuizQuestion {
                question_id: question.question_id,
                question_type: question.question_type,
                question_text: question.question_text,
                position: question.position,
                points: question.points,
                options: option_rows
                    .into_iter()
                    .map(|option| AttemptQuizOption {
                        option_id: option.option_id,
                        option_text: option.option_text,
                        position: option.position,
                    })
                    .collect(),
            })
        })
        .collect()
}
