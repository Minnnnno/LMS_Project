use std::collections::HashMap;

use actix_session::Session;
use actix_web::HttpResponse;
use chrono::{Duration, Local, NaiveDateTime};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QueryFilter,
    QueryOrder, Statement,
};

use crate::entity::{courses, quiz, quiz_answers, quiz_attempts, quiz_options, quiz_questions};
use crate::models::quiz::{SaveQuizOption, SaveQuizQuestion};
use crate::models::quiz_answers::SavedQuizAnswer;
use crate::models::quiz_attempts::{AttemptQuizOption, AttemptQuizQuestion};
use crate::services::auth_helpers::{get_role_ids, has_staff_role, is_student_only};
use crate::services::course_service::can_manage_course;

pub fn require_staff(session: &Session) -> Result<(), HttpResponse> {
    let role_ids = get_role_ids(session);
    if role_ids.is_empty() {
        return Err(HttpResponse::Unauthorized().body("You must be logged in"));
    }
    if !has_staff_role(&role_ids) {
        return Err(HttpResponse::Forbidden().body("Staff role required"));
    }
    Ok(())
}

pub fn require_student(session: &Session) -> Result<(), HttpResponse> {
    let role_ids = get_role_ids(session);
    if !is_student_only(&role_ids) {
        return Err(HttpResponse::Forbidden().body("Student role required"));
    }
    Ok(())
}

pub async fn require_can_manage_course_id(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> Result<courses::Model, HttpResponse> {
    let course = courses::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    match can_manage_course(db, session, &course).await {
        Ok(true) => Ok(course),
        Ok(false) => {
            Err(HttpResponse::Forbidden().body("You cannot manage quizzes for this course"))
        }
        Err(response) => Err(response),
    }
}

pub async fn require_can_manage_quiz(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> Result<quiz::Model, HttpResponse> {
    let quiz = quiz::Entity::find_by_id(quiz_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Quiz not found"))?;

    require_can_manage_course_id(db, session, quiz.course_id).await?;
    Ok(quiz)
}

pub async fn ensure_content_editable(
    db: &impl ConnectionTrait,
    quiz_id: i32,
) -> Result<(), HttpResponse> {
    let has_attempt = quiz_attempts::Entity::find()
        .filter(quiz_attempts::Column::QuizId.eq(quiz_id))
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?
        .is_some();

    if has_attempt {
        return Err(HttpResponse::Conflict()
            .body("Quiz content cannot be changed after attempts have started"));
    }
    Ok(())
}

pub async fn lock_quiz(db: &impl ConnectionTrait, quiz_id: i32) -> Result<(), HttpResponse> {
    advisory_lock(db, 2, quiz_id, "Quiz content").await
}

pub async fn lock_attempt(db: &impl ConnectionTrait, attempt_id: i32) -> Result<(), HttpResponse> {
    advisory_lock(db, 1, attempt_id, "Attempt").await
}

async fn advisory_lock(
    db: &impl ConnectionTrait,
    namespace: i32,
    id: i32,
    label: &str,
) -> Result<(), HttpResponse> {
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1, $2)",
        [namespace.into(), id.into()],
    ))
    .await
    .map(|_| ())
    .map_err(|err| {
        HttpResponse::InternalServerError().body(format!("{} lock error: {}", label, err))
    })
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
        .map(|expires_at| Local::now().naive_local() >= expires_at)
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

pub async fn load_editor_questions(
    db: &impl ConnectionTrait,
    quiz_id: i32,
) -> Result<Vec<SaveQuizQuestion>, HttpResponse> {
    let questions = quiz_questions::Entity::find()
        .filter(quiz_questions::Column::QuizId.eq(quiz_id))
        .order_by_asc(quiz_questions::Column::Position)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?;
    let question_ids = questions
        .iter()
        .map(|question| question.question_id)
        .collect::<Vec<_>>();
    let options = if question_ids.is_empty() {
        Vec::new()
    } else {
        quiz_options::Entity::find()
            .filter(quiz_options::Column::QuestionId.is_in(question_ids))
            .order_by_asc(quiz_options::Column::QuestionId)
            .order_by_asc(quiz_options::Column::Position)
            .all(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!("Database error: {}", err))
            })?
    };
    let mut options_by_question = HashMap::<i32, Vec<quiz_options::Model>>::new();
    for option in options {
        options_by_question
            .entry(option.question_id)
            .or_default()
            .push(option);
    }

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
) -> Result<Vec<AttemptQuizQuestion>, HttpResponse> {
    let questions = quiz_questions::Entity::find()
        .filter(quiz_questions::Column::QuizId.eq(quiz_id))
        .order_by_asc(quiz_questions::Column::Position)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?;
    if questions.is_empty() {
        return Err(HttpResponse::Conflict().body("This quiz has no questions yet"));
    }
    let question_ids = questions
        .iter()
        .map(|question| question.question_id)
        .collect::<Vec<_>>();
    let options = quiz_options::Entity::find()
        .filter(quiz_options::Column::QuestionId.is_in(question_ids))
        .order_by_asc(quiz_options::Column::QuestionId)
        .order_by_asc(quiz_options::Column::Position)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Database error: {}", err))
        })?;
    let mut options_by_question = HashMap::<i32, Vec<quiz_options::Model>>::new();
    for option in options {
        options_by_question
            .entry(option.question_id)
            .or_default()
            .push(option);
    }

    questions
        .into_iter()
        .map(|question| {
            let option_rows = options_by_question
                .remove(&question.question_id)
                .unwrap_or_default();
            if question.question_type == quiz_questions::QuestionType::Mcq
                && (option_rows.len() < 2 || !option_rows.iter().any(|option| option.is_correct))
            {
                return Err(HttpResponse::Conflict()
                    .body("This quiz has an invalid MCQ question and cannot be attempted yet"));
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
