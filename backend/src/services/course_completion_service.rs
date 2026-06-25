use actix_web::HttpResponse;
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::entity::{
    assignments, enrollments, module_progress, modules, quiz, quiz_attempts, submissions,
};
use crate::models::module_progress::CourseProgress;

#[derive(Clone, Debug, Serialize)]
pub struct CourseCompletionStatus {
    pub progress: CourseProgress,
    pub automatic_completed: bool,
    pub manual_completed: bool,
    pub completed: bool,
    pub completion_source: String,
    pub content_complete: bool,
    pub quizzes_graded: bool,
    pub assignments_graded: bool,
    pub manual_completed_at: Option<DateTime<Utc>>,
    pub manual_completed_by: Option<i32>,
    pub manual_completion_note: Option<String>,
}

fn course_progress_from_completed(total_modules: u64, completed_modules: u64) -> CourseProgress {
    CourseProgress {
        completed_modules,
        total_modules,
        progress_percent: if total_modules == 0 {
            0
        } else {
            ((completed_modules * 100) / total_modules)
                .min(100)
                .try_into()
                .unwrap_or(100)
        },
    }
}

pub async fn load_completion_statuses(
    db: &DatabaseConnection,
    enrollment_rows: &[enrollments::Model],
) -> Result<HashMap<(i32, i32), CourseCompletionStatus>, HttpResponse> {
    if enrollment_rows.is_empty() {
        return Ok(HashMap::new());
    }

    let user_ids: Vec<i32> = enrollment_rows
        .iter()
        .map(|enrollment| enrollment.user_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let course_ids: Vec<i32> = enrollment_rows
        .iter()
        .map(|enrollment| enrollment.course_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let (module_result, assignment_result, quiz_result) = tokio::join!(
        modules::Entity::find()
            .filter(modules::Column::CourseId.is_in(course_ids.clone()))
            .all(db),
        assignments::Entity::find()
            .filter(assignments::Column::CourseId.is_in(course_ids.clone()))
            .all(db),
        quiz::Entity::find()
            .filter(quiz::Column::CourseId.is_in(course_ids))
            .all(db),
    );

    let module_rows = module_result.map_err(|err| {
        HttpResponse::InternalServerError()
            .body(format!("Database error finding course modules: {}", err))
    })?;
    let assignment_rows = assignment_result.map_err(|err| {
        HttpResponse::InternalServerError().body(format!(
            "Database error finding course assignments: {}",
            err
        ))
    })?;
    let quiz_rows = quiz_result.map_err(|err| {
        HttpResponse::InternalServerError()
            .body(format!("Database error finding course quizzes: {}", err))
    })?;

    let module_ids: Vec<i32> = module_rows.iter().map(|module| module.module_id).collect();
    let assignment_ids: Vec<i32> = assignment_rows
        .iter()
        .map(|assignment| assignment.assignment_id)
        .collect();
    let quiz_ids: Vec<i32> = quiz_rows.iter().map(|quiz| quiz.quiz_id).collect();

    let progress_result = if module_ids.is_empty() {
        Ok(Vec::new())
    } else {
        module_progress::Entity::find()
            .filter(module_progress::Column::UserId.is_in(user_ids.clone()))
            .filter(module_progress::Column::ModuleId.is_in(module_ids.clone()))
            .filter(module_progress::Column::CompletedAt.is_not_null())
            .all(db)
            .await
    };
    let submission_result = if assignment_ids.is_empty() {
        Ok(Vec::new())
    } else {
        submissions::Entity::find()
            .filter(submissions::Column::UserId.is_in(user_ids.clone()))
            .filter(submissions::Column::AssignmentId.is_in(assignment_ids))
            .all(db)
            .await
    };
    let attempt_result = if quiz_ids.is_empty() {
        Ok(Vec::new())
    } else {
        quiz_attempts::Entity::find()
            .filter(quiz_attempts::Column::UserId.is_in(user_ids))
            .filter(quiz_attempts::Column::QuizId.is_in(quiz_ids.clone()))
            .all(db)
            .await
    };

    let progress_rows = progress_result.map_err(|err| {
        HttpResponse::InternalServerError()
            .body(format!("Database error finding module progress: {}", err))
    })?;
    let submission_rows = submission_result.map_err(|err| {
        HttpResponse::InternalServerError().body(format!(
            "Database error finding assignment submissions: {}",
            err
        ))
    })?;
    let attempt_rows = attempt_result.map_err(|err| {
        HttpResponse::InternalServerError()
            .body(format!("Database error finding quiz attempts: {}", err))
    })?;

    let mut module_ids_by_course: HashMap<i32, Vec<i32>> = HashMap::new();
    for module in module_rows {
        module_ids_by_course
            .entry(module.course_id)
            .or_default()
            .push(module.module_id);
    }

    let mut assignment_ids_by_course: HashMap<i32, Vec<i32>> = HashMap::new();
    for assignment in assignment_rows {
        assignment_ids_by_course
            .entry(assignment.course_id)
            .or_default()
            .push(assignment.assignment_id);
    }

    let mut quiz_ids_by_course: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut passing_mark_by_quiz: HashMap<i32, i32> = HashMap::new();
    for quiz in quiz_rows {
        passing_mark_by_quiz.insert(quiz.quiz_id, quiz.passing_mark);
        quiz_ids_by_course
            .entry(quiz.course_id)
            .or_default()
            .push(quiz.quiz_id);
    }

    let mut completed_module_ids_by_user: HashMap<i32, HashSet<i32>> = HashMap::new();
    for progress in progress_rows {
        completed_module_ids_by_user
            .entry(progress.user_id)
            .or_default()
            .insert(progress.module_id);
    }

    let mut latest_submission_by_user_assignment: HashMap<(i32, i32), submissions::Model> =
        HashMap::new();
    for submission in submission_rows {
        latest_submission_by_user_assignment
            .entry((submission.user_id, submission.assignment_id))
            .and_modify(|current| {
                if submission.submitted_at > current.submitted_at
                    || (submission.submitted_at == current.submitted_at
                        && submission.submission_id > current.submission_id)
                {
                    *current = submission.clone();
                }
            })
            .or_insert(submission);
    }

    let mut question_points_by_quiz: HashMap<i32, i32> = HashMap::new();
    let quiz_question_rows = if quiz_ids.is_empty() {
        Vec::new()
    } else {
        crate::entity::quiz_questions::Entity::find()
            .filter(crate::entity::quiz_questions::Column::QuizId.is_in(quiz_ids))
            .all(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError()
                    .body(format!("Database error finding quiz questions: {}", err))
            })?
    };
    for question in quiz_question_rows {
        *question_points_by_quiz.entry(question.quiz_id).or_insert(0) += question.points;
    }

    let mut passed_quiz_ids_by_user: HashMap<i32, HashSet<i32>> = HashMap::new();
    for attempt in attempt_rows {
        let max_score = question_points_by_quiz
            .get(&attempt.quiz_id)
            .copied()
            .unwrap_or(0);
        let passing_mark = passing_mark_by_quiz
            .get(&attempt.quiz_id)
            .copied()
            .unwrap_or(50);
        let passed = attempt.is_graded
            && attempt.submitted_at.is_some()
            && max_score > 0
            && attempt
                .total_score
                .is_some_and(|score| score * 100 >= passing_mark * max_score);

        if passed {
            passed_quiz_ids_by_user
                .entry(attempt.user_id)
                .or_default()
                .insert(attempt.quiz_id);
        }
    }

    let statuses = enrollment_rows
        .iter()
        .map(|enrollment| {
            let course_module_ids = module_ids_by_course
                .get(&enrollment.course_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let course_assignment_ids = assignment_ids_by_course
                .get(&enrollment.course_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let course_quiz_ids = quiz_ids_by_course
                .get(&enrollment.course_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let completed_module_ids = completed_module_ids_by_user
                .get(&enrollment.user_id)
                .cloned()
                .unwrap_or_default();
            let passed_quiz_ids = passed_quiz_ids_by_user
                .get(&enrollment.user_id)
                .cloned()
                .unwrap_or_default();

            let content_complete = !course_module_ids.is_empty()
                && course_module_ids
                    .iter()
                    .all(|module_id| completed_module_ids.contains(module_id));
            let assignments_graded = course_assignment_ids.iter().all(|assignment_id| {
                latest_submission_by_user_assignment
                    .get(&(enrollment.user_id, *assignment_id))
                    .is_some_and(|submission| submission.score.is_some())
            });
            let quizzes_graded = course_quiz_ids
                .iter()
                .all(|quiz_id| passed_quiz_ids.contains(quiz_id));
            let automatic_completed = content_complete && assignments_graded && quizzes_graded;
            let manual_completed = enrollment.manual_completed_at.is_some();
            let completed = automatic_completed || manual_completed;
            let completion_source = if manual_completed {
                "manual"
            } else if automatic_completed {
                "automatic"
            } else {
                "none"
            }
            .to_string();
            let progress = course_progress_from_completed(
                course_module_ids.len() as u64,
                completed_module_ids
                    .iter()
                    .filter(|module_id| course_module_ids.contains(module_id))
                    .count() as u64,
            );

            (
                (enrollment.user_id, enrollment.course_id),
                CourseCompletionStatus {
                    progress,
                    automatic_completed,
                    manual_completed,
                    completed,
                    completion_source,
                    content_complete,
                    quizzes_graded,
                    assignments_graded,
                    manual_completed_at: enrollment.manual_completed_at,
                    manual_completed_by: enrollment.manual_completed_by,
                    manual_completion_note: enrollment.manual_completion_note.clone(),
                },
            )
        })
        .collect();

    Ok(statuses)
}
