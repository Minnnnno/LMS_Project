use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Serialize;
use std::collections::HashMap;

use crate::entity::courses;
use crate::entity::quiz::{Column as QuizColumn, Entity as QuizEntity, Model as QuizModel};
use crate::entity::quiz_answers::{Column as QuizAnswerColumn, Entity as QuizAnswerEntity};
use crate::entity::quiz_attempts::{Column as QuizAttemptColumn, Entity as QuizAttemptEntity};
use crate::entity::quiz_questions::QuestionType;
use crate::services::course_service::can_manage_course;
use crate::services::quiz_helper;

const ANALYTICS_UNAVAILABLE_MESSAGE: &str =
    "All attempts must be graded before analytics are available";

#[derive(Serialize)]
struct QuizAnalyticsSummary {
    quiz_id: i32,
    title: String,
    submitted_attempts: usize,
    analytics_available: bool,
    message: String,
}

#[derive(Serialize)]
struct QuestionAnalytics {
    question_id: i32,
    position: i32,
    question_text: String,
    question_type: QuestionType,
    max_points: i32,
    correct_percentage: Option<f64>,
    average_score: Option<f64>,
}

#[derive(Serialize)]
struct QuizAnalyticsDetail {
    quiz_id: i32,
    title: String,
    submitted_attempts: usize,
    average_class_performance: f64,
    above_80_percentage: f64,
    below_40_percentage: f64,
    questions: Vec<QuestionAnalytics>,
}

async fn require_course_access(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> Result<courses::Model, HttpResponse> {
    quiz_helper::require_staff(session)?;

    let course = courses::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(quiz_helper::db_error)?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    match can_manage_course(db, session, &course).await {
        Ok(true) => Ok(course),
        Ok(false) => {
            Err(HttpResponse::Forbidden().body("You cannot view quiz analytics for this course"))
        }
        Err(response) => Err(response),
    }
}

async fn require_quiz_access(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> Result<QuizModel, HttpResponse> {
    let quiz = QuizEntity::find_by_id(quiz_id)
        .one(db)
        .await
        .map_err(quiz_helper::db_error)?
        .ok_or_else(|| HttpResponse::NotFound().body("Quiz not found"))?;

    require_course_access(db, session, quiz.course_id).await?;
    Ok(quiz)
}

pub async fn list_course_analytics(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> HttpResponse {
    if let Err(response) = require_course_access(db, session, course_id).await {
        return response;
    }

    let quizzes = match QuizEntity::find()
        .filter(QuizColumn::CourseId.eq(course_id))
        .all(db)
        .await
    {
        Ok(quizzes) => quizzes,
        Err(err) => {
            return quiz_helper::db_error(err);
        }
    };

    if quizzes.is_empty() {
        return HttpResponse::Ok().json(Vec::<QuizAnalyticsSummary>::new());
    }

    let quiz_ids = quizzes.iter().map(|quiz| quiz.quiz_id).collect::<Vec<_>>();
    let attempts = match QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::QuizId.is_in(quiz_ids))
        .filter(QuizAttemptColumn::SubmittedAt.is_not_null())
        .all(db)
        .await
    {
        Ok(attempts) => attempts,
        Err(err) => {
            return quiz_helper::db_error(err);
        }
    };

    let mut attempt_counts = HashMap::<i32, usize>::new();
    let mut ungraded_counts = HashMap::<i32, usize>::new();
    for attempt in attempts {
        *attempt_counts.entry(attempt.quiz_id).or_default() += 1;
        if !attempt.is_graded {
            *ungraded_counts.entry(attempt.quiz_id).or_default() += 1;
        }
    }

    let payload = quizzes
        .into_iter()
        .map(|quiz| {
            let submitted_attempts = attempt_counts.get(&quiz.quiz_id).copied().unwrap_or(0);
            let analytics_available = submitted_attempts > 0
                && ungraded_counts.get(&quiz.quiz_id).copied().unwrap_or(0) == 0;

            QuizAnalyticsSummary {
                quiz_id: quiz.quiz_id,
                title: quiz.title,
                submitted_attempts,
                analytics_available,
                message: if analytics_available {
                    "View analytics".to_string()
                } else {
                    ANALYTICS_UNAVAILABLE_MESSAGE.to_string()
                },
            }
        })
        .collect::<Vec<_>>();

    HttpResponse::Ok().json(payload)
}

pub async fn get_quiz_analytics(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> HttpResponse {
    let quiz = match require_quiz_access(db, session, quiz_id).await {
        Ok(quiz) => quiz,
        Err(response) => return response,
    };

    let attempts = match QuizAttemptEntity::find()
        .filter(QuizAttemptColumn::QuizId.eq(quiz_id))
        .filter(QuizAttemptColumn::SubmittedAt.is_not_null())
        .all(db)
        .await
    {
        Ok(attempts) => attempts,
        Err(err) => return quiz_helper::db_error(err),
    };

    if attempts.is_empty() || attempts.iter().any(|attempt| !attempt.is_graded) {
        return HttpResponse::Conflict().body(ANALYTICS_UNAVAILABLE_MESSAGE);
    }

    let questions = match quiz_helper::load_quiz_questions(db, quiz_id).await {
        Ok(questions) => questions,
        Err(error) => return error.into_response(),
    };

    let max_score = questions
        .iter()
        .map(|question| question.points)
        .sum::<i32>();
    if max_score <= 0 {
        return HttpResponse::Conflict().body("Quiz analytics require questions with points");
    }

    let attempt_ids = attempts
        .iter()
        .map(|attempt| attempt.attempt_id)
        .collect::<Vec<_>>();
    let answers = match QuizAnswerEntity::find()
        .filter(QuizAnswerColumn::AttemptId.is_in(attempt_ids))
        .all(db)
        .await
    {
        Ok(answers) => answers,
        Err(err) => {
            return quiz_helper::db_error(err);
        }
    };

    let attempt_count = attempts.len();
    let mut performance_total = 0.0;
    let mut above_80_count = 0usize;
    let mut below_40_count = 0usize;
    for attempt in &attempts {
        let percentage = attempt.total_score.unwrap_or(0) as f64 / max_score as f64 * 100.0;
        performance_total += percentage;
        if percentage > 80.0 {
            above_80_count += 1;
        }
        if percentage < 40.0 {
            below_40_count += 1;
        }
    }

    let question_points = questions
        .iter()
        .map(|question| (question.question_id, question.points))
        .collect::<HashMap<_, _>>();
    let mut answer_scores = HashMap::<(i32, i32), i32>::new();
    for answer in answers {
        if !question_points.contains_key(&answer.question_id) {
            continue;
        }
        answer_scores.insert(
            (answer.attempt_id, answer.question_id),
            answer.score.unwrap_or(0),
        );
    }

    let mut question_score_totals = HashMap::<i32, i64>::new();
    let mut question_full_score_counts = HashMap::<i32, usize>::new();
    for ((_, question_id), score) in answer_scores {
        *question_score_totals.entry(question_id).or_default() += score as i64;
        if question_points.get(&question_id) == Some(&score) {
            *question_full_score_counts.entry(question_id).or_default() += 1;
        }
    }

    let question_analytics = questions
        .into_iter()
        .map(|question| {
            let score_total = question_score_totals
                .get(&question.question_id)
                .copied()
                .unwrap_or(0);
            let full_score_count = question_full_score_counts
                .get(&question.question_id)
                .copied()
                .unwrap_or(0);
            let is_mcq = question.question_type == QuestionType::Mcq;

            QuestionAnalytics {
                question_id: question.question_id,
                position: question.position,
                question_text: question.question_text,
                question_type: question.question_type,
                max_points: question.points,
                correct_percentage: is_mcq
                    .then(|| full_score_count as f64 / attempt_count as f64 * 100.0),
                average_score: (!is_mcq).then(|| score_total as f64 / attempt_count as f64),
            }
        })
        .collect();

    HttpResponse::Ok().json(QuizAnalyticsDetail {
        quiz_id,
        title: quiz.title,
        submitted_attempts: attempt_count,
        average_class_performance: performance_total / attempt_count as f64,
        above_80_percentage: above_80_count as f64 / attempt_count as f64 * 100.0,
        below_40_percentage: below_40_count as f64 / attempt_count as f64 * 100.0,
        questions: question_analytics,
    })
}
