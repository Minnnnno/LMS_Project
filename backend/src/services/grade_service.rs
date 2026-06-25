use std::collections::HashMap;

use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::entity::{assignments, quiz, quiz_attempts, quiz_questions, submissions};
use crate::models::grade::{AssignmentGrade, CourseGradebook, QuizGrade};
use crate::services::auth_helpers::{get_user_id, is_enrolled};

fn passed_quiz(
    total_score: Option<i32>,
    max_score: i32,
    passing_mark: i32,
    is_graded: bool,
) -> Option<bool> {
    if !is_graded || max_score <= 0 {
        return None;
    }

    total_score.map(|score| score * 100 >= passing_mark * max_score)
}

pub async fn get_my_course_grades(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match is_enrolled(db, user_id, course_id).await {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Forbidden()
                .body("You must be enrolled in this course to view grades");
        }
        Err(response) => return response,
    }

    let course_assignments = match assignments::Entity::find()
        .filter(assignments::Column::CourseId.eq(course_id))
        .all(db)
        .await
    {
        Ok(items) => items,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding assignments: {}", err));
        }
    };

    let assignment_ids = course_assignments
        .iter()
        .map(|assignment| assignment.assignment_id)
        .collect::<Vec<_>>();

    let assignment_submissions = if assignment_ids.is_empty() {
        Vec::new()
    } else {
        match submissions::Entity::find()
            .filter(submissions::Column::UserId.eq(user_id))
            .filter(submissions::Column::AssignmentId.is_in(assignment_ids))
            .all(db)
            .await
        {
            Ok(items) => items,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding submissions: {}", err));
            }
        }
    };

    let mut latest_submission_by_assignment = HashMap::new();
    for submission in assignment_submissions {
        latest_submission_by_assignment
            .entry(submission.assignment_id)
            .and_modify(|current: &mut submissions::Model| {
                if submission.submitted_at > current.submitted_at {
                    *current = submission.clone();
                }
            })
            .or_insert(submission);
    }

    let assignment_grades = course_assignments
        .into_iter()
        .map(|assignment| {
            let submission = latest_submission_by_assignment.get(&assignment.assignment_id);

            AssignmentGrade {
                assignment_id: assignment.assignment_id,
                title: assignment.title,
                due_date: assignment.due_date,
                max_score: assignment.max_score,
                score: submission.and_then(|item| item.score),
                feedback: submission
                    .and_then(|item| item.feedback.clone())
                    .filter(|feedback| !feedback.trim().is_empty()),
                submitted_at: submission.map(|item| item.submitted_at),
            }
        })
        .collect::<Vec<_>>();

    let mut quiz_message = None;
    let course_quizzes = match quiz::Entity::find()
        .filter(quiz::Column::CourseId.eq(course_id))
        .all(db)
        .await
    {
        Ok(items) => items,
        Err(err) => {
            eprintln!("Database error finding quizzes for grades: {}", err);
            quiz_message = Some("Quiz grades are not available yet.".to_string());
            Vec::new()
        }
    };

    let quiz_ids = course_quizzes
        .iter()
        .map(|quiz| quiz.quiz_id)
        .collect::<Vec<_>>();

    let questions = if quiz_ids.is_empty() {
        Vec::new()
    } else {
        match quiz_questions::Entity::find()
            .filter(quiz_questions::Column::QuizId.is_in(quiz_ids.clone()))
            .all(db)
            .await
        {
            Ok(items) => items,
            Err(err) => {
                eprintln!("Database error finding quiz questions for grades: {}", err);
                quiz_message = Some("Quiz grades are not available yet.".to_string());
                Vec::new()
            }
        }
    };

    let mut max_score_by_quiz = HashMap::new();
    for question in questions {
        *max_score_by_quiz.entry(question.quiz_id).or_insert(0) += question.points;
    }

    let attempts = if quiz_ids.is_empty() {
        Vec::new()
    } else {
        match quiz_attempts::Entity::find()
            .filter(quiz_attempts::Column::UserId.eq(user_id))
            .filter(quiz_attempts::Column::QuizId.is_in(quiz_ids))
            .all(db)
            .await
        {
            Ok(items) => items,
            Err(err) => {
                eprintln!("Database error finding quiz attempts for grades: {}", err);
                quiz_message = Some("Quiz grades are not available yet.".to_string());
                Vec::new()
            }
        }
    };

    let mut latest_graded_attempt_by_quiz = HashMap::new();
    let mut latest_attempt_by_quiz = HashMap::new();
    for attempt in attempts {
        if attempt.is_graded {
            latest_graded_attempt_by_quiz
                .entry(attempt.quiz_id)
                .and_modify(|current: &mut quiz_attempts::Model| {
                    let current_time = current.submitted_at.unwrap_or(current.started_at);
                    let attempt_time = attempt.submitted_at.unwrap_or(attempt.started_at);

                    if attempt_time > current_time {
                        *current = attempt.clone();
                    }
                })
                .or_insert_with(|| attempt.clone());
        }

        latest_attempt_by_quiz
            .entry(attempt.quiz_id)
            .and_modify(|current: &mut quiz_attempts::Model| {
                let replace_current = match (current.submitted_at, attempt.submitted_at) {
                    (Some(current_submitted_at), Some(attempt_submitted_at)) => {
                        attempt_submitted_at > current_submitted_at
                    }
                    (None, Some(_)) => true,
                    (Some(_), None) => false,
                    (None, None) => attempt.started_at > current.started_at,
                };

                if replace_current {
                    *current = attempt.clone();
                }
            })
            .or_insert(attempt);
    }

    let quiz_grades = course_quizzes
        .into_iter()
        .map(|quiz| {
            let attempt = latest_graded_attempt_by_quiz
                .get(&quiz.quiz_id)
                .or_else(|| latest_attempt_by_quiz.get(&quiz.quiz_id));
            let max_score = *max_score_by_quiz.get(&quiz.quiz_id).unwrap_or(&0);
            let total_score =
                attempt.and_then(|item| item.is_graded.then_some(item.total_score).flatten());
            let is_graded = attempt.map(|item| item.is_graded).unwrap_or(false);

            QuizGrade {
                quiz_id: quiz.quiz_id,
                title: quiz.title,
                max_score,
                passing_mark: quiz.passing_mark,
                total_score,
                submitted_at: attempt.and_then(|item| item.submitted_at),
                attempt_id: attempt.map(|item| item.attempt_id),
                is_graded,
                passed: passed_quiz(total_score, max_score, quiz.passing_mark, is_graded),
            }
        })
        .collect::<Vec<_>>();

    HttpResponse::Ok().json(CourseGradebook {
        assignments: assignment_grades,
        quizzes: quiz_grades,
        quiz_message,
    })
}
