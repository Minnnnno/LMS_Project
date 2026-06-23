use crate::entity::courses::{self, CourseStatus};
use crate::entity::enrollments;
use crate::entity::{
    assignments, module_contents, module_prerequisites, module_progress, modules, quiz,
    quiz_attempts, quiz_prerequisites, users,
};
use crate::models::course::{CourseQuery, CreateCourse, UpdateCourse};
use crate::models::module_progress::{CourseModuleProgress, CourseProgress};
use crate::services::certificate_service::revoke_certificate_if_incomplete;
use crate::services::course_completion_service::{
    CourseCompletionStatus, load_completion_statuses,
};
use crate::services::course_service::{
    can_manage_course, can_view_course, get_instructor_course_ids_for_session,
    get_instructor_courses_for_session, get_organisation_courses_for_session,
    get_session_user_org_id, has_role, is_instructor_course_limited, normalize_course_visibility,
    price_to_cents,
};
use actix_session::Session;
use actix_web::{HttpResponse, Responder, delete, get, post, put, web};
use chrono::{Local, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize)]
struct ModuleOverviewPayload {
    module_id: i32,
    course_id: i32,
    title: String,
    position: i32,
    prerequisite_module_ids: Vec<i32>,
}

#[derive(Serialize)]
struct QuizOverviewPayload {
    quiz_id: i32,
    course_id: i32,
    title: String,
    description: Option<String>,
    max_attempts: Option<i32>,
    time_limit: Option<i32>,
    starts_at: Option<chrono::NaiveDateTime>,
    ends_at: Option<chrono::NaiveDateTime>,
    created_at: chrono::NaiveDateTime,
    prerequisite_module_ids: Vec<i32>,
}

#[derive(Serialize)]
struct CourseOverviewPayload {
    course: courses::Model,
    can_manage: bool,
    enrolled: bool,
    modules: Vec<ModuleOverviewPayload>,
    assignments: Vec<assignments::Model>,
    quizzes: Vec<QuizOverviewPayload>,
    course_progress: Option<CourseProgress>,
    module_progress: Vec<CourseModuleProgress>,
}

#[derive(Serialize)]
struct CourseProgressOverviewPayload {
    course: courses::Model,
    progress: CourseProgress,
}

#[derive(Serialize)]
struct CourseCompletionOverviewPayload {
    course: courses::Model,
    completed: bool,
    automatic_completed: bool,
    manual_completed: bool,
    completion_source: String,
    content_complete: bool,
    quizzes_graded: bool,
    assignments_graded: bool,
    manual_completed_at: Option<chrono::DateTime<chrono::Utc>>,
    manual_completed_by: Option<i32>,
    manual_completion_note: Option<String>,
    progress: CourseProgress,
}

#[derive(Serialize)]
struct CourseCompletionRosterItem {
    user_id: i32,
    student_name: String,
    student_email: String,
    status: CourseCompletionStatus,
}

#[derive(Deserialize)]
struct ManualCompletionRequest {
    note: Option<String>,
}

#[derive(Serialize)]
struct CourseAssignmentsOverviewPayload {
    course: courses::Model,
    assignments: Vec<assignments::Model>,
}

#[derive(Serialize)]
struct CourseModuleContentOverviewPayload {
    module: ModuleOverviewPayload,
    items: Vec<module_contents::Model>,
}

#[derive(Serialize)]
struct CourseContentOverviewPayload {
    course: courses::Model,
    modules: Vec<CourseModuleContentOverviewPayload>,
}

#[derive(Serialize)]
struct QuizAttemptStatusOverviewPayload {
    quiz_id: i32,
    attempts_used: usize,
    attempts_left: Option<i32>,
    max_attempts: Option<i32>,
    has_submitted_attempt: bool,
    can_attempt: bool,
    message: String,
}

#[derive(Serialize)]
struct CourseAssessmentsOverviewPayload {
    course: courses::Model,
    quizzes: Vec<QuizOverviewPayload>,
    statuses: Vec<QuizAttemptStatusOverviewPayload>,
}

async fn accessible_course_condition(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<Option<Condition>, HttpResponse> {
    if has_role(session, "LMS Admin") {
        return Ok(None);
    }

    let user_org_id = match session.get::<i32>("user_id") {
        Ok(Some(_)) => match get_session_user_org_id(db, session).await {
            Ok(org_id) => org_id,
            Err(response) => return Err(response),
        },
        Ok(None) => None,
        Err(err) => {
            return Err(HttpResponse::InternalServerError().body(format!("Session error: {}", err)));
        }
    };

    let mut condition = Condition::any().add(courses::Column::Visibility.eq("public"));

    if let Some(org_id) = user_org_id {
        condition = condition.add(
            Condition::all()
                .add(courses::Column::Visibility.eq("private"))
                .add(courses::Column::OrgId.eq(org_id)),
        );
    }

    Ok(Some(condition))
}

fn session_user_id(session: &Session) -> Result<i32, HttpResponse> {
    match session.get::<i32>("user_id") {
        Ok(Some(id)) => Ok(id),
        Ok(None) => Err(HttpResponse::Unauthorized().body("User not logged in")),
        Err(err) => {
            Err(HttpResponse::InternalServerError().body(format!("Session error: {}", err)))
        }
    }
}

async fn require_manageable_course(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> Result<courses::Model, HttpResponse> {
    let course = courses::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    match can_manage_course(db, session, &course).await {
        Ok(true) => Ok(course),
        Ok(false) => Err(HttpResponse::Forbidden().body("You cannot manage this course")),
        Err(response) => Err(response),
    }
}

fn normalize_manual_completion_note(note: Option<String>) -> Result<Option<String>, HttpResponse> {
    let Some(note) = note else {
        return Ok(None);
    };

    let note = note.trim().to_string();
    if note.is_empty() {
        return Ok(None);
    }

    if note.chars().count() > 500 {
        return Err(
            HttpResponse::BadRequest().body("Completion note must be 500 characters or fewer")
        );
    }

    Ok(Some(note))
}

async fn get_enrolled_courses(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<(i32, Vec<courses::Model>), HttpResponse> {
    let user_id = session_user_id(session)?;

    let enrollment_rows = enrollments::Entity::find()
        .filter(enrollments::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding enrollments: {}", err))
        })?;

    let course_ids: Vec<i32> = enrollment_rows
        .into_iter()
        .map(|enrollment| enrollment.course_id)
        .collect();

    if course_ids.is_empty() {
        return Ok((user_id, Vec::new()));
    }

    let courses = courses::Entity::find()
        .filter(courses::Column::CourseId.is_in(course_ids))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding courses: {}", err))
        })?;

    Ok((user_id, courses))
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

fn build_quiz_attempt_status(
    quiz: &quiz::Model,
    attempts: &[quiz_attempts::Model],
) -> QuizAttemptStatusOverviewPayload {
    let attempts_used = attempts.len();
    let attempts_left = quiz
        .max_attempts
        .map(|max| (max - attempts_used as i32).max(0));
    let has_submitted_attempt = attempts
        .iter()
        .any(|attempt| attempt.submitted_at.is_some());

    if quiz
        .starts_at
        .is_some_and(|starts_at| starts_at > Local::now().naive_local())
    {
        return QuizAttemptStatusOverviewPayload {
            quiz_id: quiz.quiz_id,
            attempts_used,
            attempts_left,
            max_attempts: quiz.max_attempts,
            has_submitted_attempt,
            can_attempt: false,
            message: "This quiz is not open yet".to_string(),
        };
    }

    if quiz
        .ends_at
        .is_some_and(|ends_at| ends_at < Local::now().naive_local())
    {
        return QuizAttemptStatusOverviewPayload {
            quiz_id: quiz.quiz_id,
            attempts_used,
            attempts_left,
            max_attempts: quiz.max_attempts,
            has_submitted_attempt,
            can_attempt: false,
            message: "This quiz is closed".to_string(),
        };
    }

    if attempts_left == Some(0) {
        return QuizAttemptStatusOverviewPayload {
            quiz_id: quiz.quiz_id,
            attempts_used,
            attempts_left,
            max_attempts: quiz.max_attempts,
            has_submitted_attempt,
            can_attempt: false,
            message: "No attempts left".to_string(),
        };
    }

    let message = match attempts_left {
        Some(1) => "1 attempt left".to_string(),
        Some(left) => format!("{} attempts left", left),
        None if has_submitted_attempt => "Attempted".to_string(),
        None => "Unlimited attempts".to_string(),
    };

    QuizAttemptStatusOverviewPayload {
        quiz_id: quiz.quiz_id,
        attempts_used,
        attempts_left,
        max_attempts: quiz.max_attempts,
        has_submitted_attempt,
        can_attempt: true,
        message,
    }
}

#[get("/courses")]
pub async fn get_courses(db: web::Data<DatabaseConnection>, session: Session) -> impl Responder {
    if is_instructor_course_limited(&session) {
        return match get_instructor_courses_for_session(db.get_ref(), &session).await {
            Ok(courses) => HttpResponse::Ok().json(courses),
            Err(response) => response,
        };
    }

    let mut query = courses::Entity::find();

    if let Some(condition) = match accessible_course_condition(db.get_ref(), &session).await {
        Ok(condition) => condition,
        Err(response) => return response,
    } {
        query = query.filter(condition);
    }

    let result = query.all(db.get_ref()).await;
    match result {
        Ok(course) => {
            if course.is_empty() {
                HttpResponse::NotFound().body("No courses found")
            } else {
                HttpResponse::Ok().json(course)
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[get("/my-courses")]
pub async fn get_my_courses(db: web::Data<DatabaseConnection>, session: Session) -> impl Responder {
    if is_instructor_course_limited(&session) {
        return match get_instructor_courses_for_session(db.get_ref(), &session).await {
            Ok(courses) => HttpResponse::Ok().json(courses),
            Err(response) => response,
        };
    }

    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::Unauthorized().body("User not logged in");
        }
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Session error: {}", err));
        }
    };

    let enrollment_rows = match enrollments::Entity::find()
        .filter(enrollments::Column::UserId.eq(user_id))
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding enrollments: {}", err));
        }
    };

    let course_ids: Vec<i32> = enrollment_rows
        .into_iter()
        .map(|enrollment| enrollment.course_id)
        .collect();

    if course_ids.is_empty() {
        return HttpResponse::Ok().json(Vec::<courses::Model>::new());
    }

    match courses::Entity::find()
        .filter(courses::Column::CourseId.is_in(course_ids))
        .all(db.get_ref())
        .await
    {
        Ok(courses) => HttpResponse::Ok().json(courses),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error finding courses: {}", err)),
    }
}

#[get("/my-courses/progress-overview")]
pub async fn get_my_courses_progress_overview(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let (user_id, course_rows) = match get_enrolled_courses(db.get_ref(), &session).await {
        Ok(result) => result,
        Err(response) => return response,
    };

    if course_rows.is_empty() {
        return HttpResponse::Ok().json(Vec::<CourseProgressOverviewPayload>::new());
    }

    let course_ids: Vec<i32> = course_rows.iter().map(|course| course.course_id).collect();
    let module_rows = match modules::Entity::find()
        .filter(modules::Column::CourseId.is_in(course_ids))
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding modules: {}", err));
        }
    };

    let module_ids: Vec<i32> = module_rows.iter().map(|module| module.module_id).collect();
    let completed_ids: HashSet<i32> = if module_ids.is_empty() {
        HashSet::new()
    } else {
        match module_progress::Entity::find()
            .filter(module_progress::Column::UserId.eq(user_id))
            .filter(module_progress::Column::ModuleId.is_in(module_ids))
            .filter(module_progress::Column::CompletedAt.is_not_null())
            .all(db.get_ref())
            .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(|progress| progress.module_id)
                .collect(),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding module progress: {}", err));
            }
        }
    };

    let mut totals_by_course: HashMap<i32, u64> = HashMap::new();
    let mut completed_by_course: HashMap<i32, u64> = HashMap::new();

    for module in module_rows {
        *totals_by_course.entry(module.course_id).or_default() += 1;

        if completed_ids.contains(&module.module_id) {
            *completed_by_course.entry(module.course_id).or_default() += 1;
        }
    }

    let payloads: Vec<CourseProgressOverviewPayload> = course_rows
        .into_iter()
        .map(|course| {
            let total_modules = *totals_by_course.get(&course.course_id).unwrap_or(&0);
            let completed_modules = *completed_by_course.get(&course.course_id).unwrap_or(&0);

            CourseProgressOverviewPayload {
                course,
                progress: course_progress_from_completed(total_modules, completed_modules),
            }
        })
        .collect();

    HttpResponse::Ok().json(payloads)
}

#[get("/my-courses/completion-overview")]
pub async fn get_my_courses_completion_overview(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let (user_id, course_rows) = match get_enrolled_courses(db.get_ref(), &session).await {
        Ok(result) => result,
        Err(response) => return response,
    };

    if course_rows.is_empty() {
        return HttpResponse::Ok().json(Vec::<CourseCompletionOverviewPayload>::new());
    }

    let course_ids: Vec<i32> = course_rows.iter().map(|course| course.course_id).collect();
    let enrollment_rows = match enrollments::Entity::find()
        .filter(enrollments::Column::UserId.eq(user_id))
        .filter(enrollments::Column::CourseId.is_in(course_ids))
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding enrollments: {}", err));
        }
    };
    let completion_statuses = match load_completion_statuses(db.get_ref(), &enrollment_rows).await {
        Ok(statuses) => statuses,
        Err(response) => return response,
    };

    let payloads = course_rows
        .into_iter()
        .filter_map(|course| {
            completion_statuses
                .get(&(user_id, course.course_id))
                .map(|status| CourseCompletionOverviewPayload {
                    course,
                    completed: status.completed,
                    automatic_completed: status.automatic_completed,
                    manual_completed: status.manual_completed,
                    completion_source: status.completion_source.clone(),
                    content_complete: status.content_complete,
                    quizzes_graded: status.quizzes_graded,
                    assignments_graded: status.assignments_graded,
                    manual_completed_at: status.manual_completed_at,
                    manual_completed_by: status.manual_completed_by,
                    manual_completion_note: status.manual_completion_note.clone(),
                    progress: status.progress.clone(),
                })
        })
        .collect::<Vec<_>>();

    HttpResponse::Ok().json(payloads)
}

#[get("/courses/{course_id}/completion-roster")]
pub async fn get_course_completion_roster(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let course_id = path.into_inner();
    if let Err(response) = require_manageable_course(db.get_ref(), &session, course_id).await {
        return response;
    }

    let enrollment_rows = match enrollments::Entity::find()
        .filter(enrollments::Column::CourseId.eq(course_id))
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding enrollments: {}", err));
        }
    };

    if enrollment_rows.is_empty() {
        return HttpResponse::Ok().json(Vec::<CourseCompletionRosterItem>::new());
    }

    let completion_statuses = match load_completion_statuses(db.get_ref(), &enrollment_rows).await {
        Ok(statuses) => statuses,
        Err(response) => return response,
    };
    let user_ids: Vec<i32> = enrollment_rows
        .iter()
        .map(|enrollment| enrollment.user_id)
        .collect();
    let user_rows = match users::Entity::find()
        .filter(users::Column::UserId.is_in(user_ids))
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding users: {}", err));
        }
    };
    let users_by_id: HashMap<i32, users::Model> = user_rows
        .into_iter()
        .map(|user| (user.user_id, user))
        .collect();

    let mut roster = enrollment_rows
        .into_iter()
        .filter_map(|enrollment| {
            let user = users_by_id.get(&enrollment.user_id)?;
            let status = completion_statuses
                .get(&(enrollment.user_id, enrollment.course_id))?
                .clone();

            Some(CourseCompletionRosterItem {
                user_id: enrollment.user_id,
                student_name: format!("{} {}", user.first_name, user.last_name)
                    .trim()
                    .to_string(),
                student_email: user.email.clone(),
                status,
            })
        })
        .collect::<Vec<_>>();

    roster.sort_by(|a, b| {
        a.student_name
            .to_lowercase()
            .cmp(&b.student_name.to_lowercase())
            .then_with(|| a.student_email.cmp(&b.student_email))
    });

    HttpResponse::Ok().json(roster)
}

#[put("/courses/{course_id}/completions/{user_id}/manual")]
pub async fn mark_course_manual_completion(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32)>,
    body: web::Json<ManualCompletionRequest>,
) -> impl Responder {
    let (course_id, user_id) = path.into_inner();
    let staff_user_id = match session_user_id(&session) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    if let Err(response) = require_manageable_course(db.get_ref(), &session, course_id).await {
        return response;
    }
    let note = match normalize_manual_completion_note(body.into_inner().note) {
        Ok(note) => note,
        Err(response) => return response,
    };

    let enrollment = match enrollments::Entity::find_by_id((user_id, course_id))
        .one(db.get_ref())
        .await
    {
        Ok(Some(enrollment)) => enrollment,
        Ok(None) => return HttpResponse::NotFound().body("Enrollment not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding enrollment: {}", err));
        }
    };

    let mut active_enrollment = enrollment.into_active_model();
    active_enrollment.manual_completed_at = Set(Some(Utc::now()));
    active_enrollment.manual_completed_by = Set(Some(staff_user_id));
    active_enrollment.manual_completion_note = Set(note);

    match active_enrollment.update(db.get_ref()).await {
        Ok(saved) => HttpResponse::Ok().json(saved),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error marking course complete: {}", err)),
    }
}

#[delete("/courses/{course_id}/completions/{user_id}/manual")]
pub async fn undo_course_manual_completion(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32)>,
) -> impl Responder {
    let (course_id, user_id) = path.into_inner();
    if let Err(response) = require_manageable_course(db.get_ref(), &session, course_id).await {
        return response;
    }

    let enrollment = match enrollments::Entity::find_by_id((user_id, course_id))
        .one(db.get_ref())
        .await
    {
        Ok(Some(enrollment)) => enrollment,
        Ok(None) => return HttpResponse::NotFound().body("Enrollment not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding enrollment: {}", err));
        }
    };

    let mut active_enrollment = enrollment.into_active_model();
    active_enrollment.manual_completed_at = Set(None);
    active_enrollment.manual_completed_by = Set(None);
    active_enrollment.manual_completion_note = Set(None);

    match active_enrollment.update(db.get_ref()).await {
        Ok(saved) => {
            if let Err(response) = revoke_certificate_if_incomplete(db.get_ref(), &saved).await {
                return response;
            }
            HttpResponse::Ok().json(saved)
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error undoing manual completion: {}", err)),
    }
}

#[get("/my-courses/assignments-overview")]
pub async fn get_my_courses_assignments_overview(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let (_, course_rows) = match get_enrolled_courses(db.get_ref(), &session).await {
        Ok(result) => result,
        Err(response) => return response,
    };

    if course_rows.is_empty() {
        return HttpResponse::Ok().json(Vec::<CourseAssignmentsOverviewPayload>::new());
    }

    let course_ids: Vec<i32> = course_rows.iter().map(|course| course.course_id).collect();
    let assignment_rows = match assignments::Entity::find()
        .filter(assignments::Column::CourseId.is_in(course_ids))
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding assignments: {}", err));
        }
    };

    let mut assignments_by_course: HashMap<i32, Vec<assignments::Model>> = HashMap::new();
    for assignment in assignment_rows {
        assignments_by_course
            .entry(assignment.course_id)
            .or_default()
            .push(assignment);
    }

    let payloads: Vec<CourseAssignmentsOverviewPayload> = course_rows
        .into_iter()
        .map(|course| CourseAssignmentsOverviewPayload {
            assignments: assignments_by_course
                .remove(&course.course_id)
                .unwrap_or_default(),
            course,
        })
        .collect();

    HttpResponse::Ok().json(payloads)
}

#[get("/my-courses/content-overview")]
pub async fn get_my_courses_content_overview(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let (_, course_rows) = match get_enrolled_courses(db.get_ref(), &session).await {
        Ok(result) => result,
        Err(response) => return response,
    };

    if course_rows.is_empty() {
        return HttpResponse::Ok().json(Vec::<CourseContentOverviewPayload>::new());
    }

    let course_ids: Vec<i32> = course_rows.iter().map(|course| course.course_id).collect();
    let module_rows = match modules::Entity::find()
        .filter(modules::Column::CourseId.is_in(course_ids))
        .order_by_asc(modules::Column::Position)
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding modules: {}", err));
        }
    };

    let module_ids: Vec<i32> = module_rows.iter().map(|module| module.module_id).collect();
    let content_rows = if module_ids.is_empty() {
        Vec::new()
    } else {
        match module_contents::Entity::find()
            .filter(module_contents::Column::ModuleId.is_in(module_ids))
            .all(db.get_ref())
            .await
        {
            Ok(rows) => rows,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding module content: {}", err));
            }
        }
    };

    let mut content_by_module: HashMap<i32, Vec<module_contents::Model>> = HashMap::new();
    for content in content_rows {
        content_by_module
            .entry(content.module_id)
            .or_default()
            .push(content);
    }

    let mut modules_by_course: HashMap<i32, Vec<CourseModuleContentOverviewPayload>> =
        HashMap::new();

    for module in module_rows {
        modules_by_course.entry(module.course_id).or_default().push(
            CourseModuleContentOverviewPayload {
                items: content_by_module
                    .remove(&module.module_id)
                    .unwrap_or_default(),
                module: ModuleOverviewPayload {
                    module_id: module.module_id,
                    course_id: module.course_id,
                    title: module.title,
                    position: module.position,
                    prerequisite_module_ids: Vec::new(),
                },
            },
        );
    }

    let payloads: Vec<CourseContentOverviewPayload> = course_rows
        .into_iter()
        .map(|course| CourseContentOverviewPayload {
            modules: modules_by_course
                .remove(&course.course_id)
                .unwrap_or_default(),
            course,
        })
        .collect();

    HttpResponse::Ok().json(payloads)
}

#[get("/my-courses/assessments-overview")]
pub async fn get_my_courses_assessments_overview(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let (user_id, course_rows) = match get_enrolled_courses(db.get_ref(), &session).await {
        Ok(result) => result,
        Err(response) => return response,
    };

    if course_rows.is_empty() {
        return HttpResponse::Ok().json(Vec::<CourseAssessmentsOverviewPayload>::new());
    }

    let course_ids: Vec<i32> = course_rows.iter().map(|course| course.course_id).collect();
    let module_rows = match modules::Entity::find()
        .filter(modules::Column::CourseId.is_in(course_ids.clone()))
        .order_by_asc(modules::Column::Position)
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding modules: {}", err));
        }
    };

    let quiz_rows = match quiz::Entity::find()
        .filter(quiz::Column::CourseId.is_in(course_ids))
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding quizzes: {}", err));
        }
    };

    let quiz_ids: Vec<i32> = quiz_rows.iter().map(|quiz| quiz.quiz_id).collect();
    let module_ids: Vec<i32> = module_rows.iter().map(|module| module.module_id).collect();

    let quiz_prerequisite_rows = if quiz_ids.is_empty() {
        Vec::new()
    } else {
        match quiz_prerequisites::Entity::find()
            .filter(quiz_prerequisites::Column::QuizId.is_in(quiz_ids.clone()))
            .order_by_asc(quiz_prerequisites::Column::PrerequisiteId)
            .all(db.get_ref())
            .await
        {
            Ok(rows) => rows,
            Err(err) => {
                return HttpResponse::InternalServerError().body(format!(
                    "Database error finding quiz prerequisites: {}",
                    err
                ));
            }
        }
    };

    let completed_ids: HashSet<i32> = if module_ids.is_empty() {
        HashSet::new()
    } else {
        match module_progress::Entity::find()
            .filter(module_progress::Column::UserId.eq(user_id))
            .filter(module_progress::Column::ModuleId.is_in(module_ids))
            .filter(module_progress::Column::CompletedAt.is_not_null())
            .all(db.get_ref())
            .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(|progress| progress.module_id)
                .collect(),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding module progress: {}", err));
            }
        }
    };

    let attempt_rows = if quiz_ids.is_empty() {
        Vec::new()
    } else {
        match quiz_attempts::Entity::find()
            .filter(quiz_attempts::Column::UserId.eq(user_id))
            .filter(quiz_attempts::Column::QuizId.is_in(quiz_ids))
            .all(db.get_ref())
            .await
        {
            Ok(rows) => rows,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding quiz attempts: {}", err));
            }
        }
    };

    let mut module_by_id: HashMap<i32, modules::Model> = HashMap::new();
    for module in module_rows {
        module_by_id.insert(module.module_id, module);
    }

    let mut quiz_prerequisites_by_id: HashMap<i32, Vec<i32>> = HashMap::new();
    for row in quiz_prerequisite_rows {
        quiz_prerequisites_by_id
            .entry(row.quiz_id)
            .or_default()
            .push(row.required_module_id);
    }

    let mut attempts_by_quiz: HashMap<i32, Vec<quiz_attempts::Model>> = HashMap::new();
    for attempt in attempt_rows {
        attempts_by_quiz
            .entry(attempt.quiz_id)
            .or_default()
            .push(attempt);
    }

    let mut quizzes_by_course: HashMap<i32, Vec<QuizOverviewPayload>> = HashMap::new();
    let mut statuses_by_course: HashMap<i32, Vec<QuizAttemptStatusOverviewPayload>> =
        HashMap::new();

    for quiz in quiz_rows {
        let prerequisite_module_ids = quiz_prerequisites_by_id
            .remove(&quiz.quiz_id)
            .unwrap_or_default();
        let attempts = attempts_by_quiz.remove(&quiz.quiz_id).unwrap_or_default();
        let mut status = build_quiz_attempt_status(&quiz, &attempts);

        let first_incomplete_prerequisite = prerequisite_module_ids
            .iter()
            .filter(|module_id| !completed_ids.contains(module_id))
            .filter_map(|module_id| module_by_id.get(module_id))
            .min_by_key(|module| module.position);

        if let Some(module) = first_incomplete_prerequisite {
            status.can_attempt = false;
            status.message = format!("Complete {} before attempting this quiz", module.title);
        }

        statuses_by_course
            .entry(quiz.course_id)
            .or_default()
            .push(status);

        quizzes_by_course
            .entry(quiz.course_id)
            .or_default()
            .push(QuizOverviewPayload {
                quiz_id: quiz.quiz_id,
                course_id: quiz.course_id,
                title: quiz.title,
                description: quiz.description,
                max_attempts: quiz.max_attempts,
                time_limit: quiz.time_limit,
                starts_at: quiz.starts_at,
                ends_at: quiz.ends_at,
                created_at: quiz.created_at,
                prerequisite_module_ids,
            });
    }

    let payloads: Vec<CourseAssessmentsOverviewPayload> = course_rows
        .into_iter()
        .map(|course| CourseAssessmentsOverviewPayload {
            quizzes: quizzes_by_course
                .remove(&course.course_id)
                .unwrap_or_default(),
            statuses: statuses_by_course
                .remove(&course.course_id)
                .unwrap_or_default(),
            course,
        })
        .collect();

    HttpResponse::Ok().json(payloads)
}

#[get("/courses/organisation")]
pub async fn get_organisation_courses(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    match get_organisation_courses_for_session(db.get_ref(), &session).await {
        Ok(courses) => HttpResponse::Ok().json(courses),
        Err(response) => response,
    }
}

#[get("/courses/{course_id}/overview")]
pub async fn get_course_overview(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let course_id = path.into_inner();
    let course = match courses::Entity::find_by_id(course_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(course)) => course,
        Ok(None) => return HttpResponse::NotFound().body("Course not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err));
        }
    };

    if is_instructor_course_limited(&session) {
        match can_manage_course(db.get_ref(), &session, &course).await {
            Ok(true) => {}
            Ok(false) => {
                return HttpResponse::Forbidden().body("You can only view courses assigned to you");
            }
            Err(response) => return response,
        }
    }

    match can_view_course(db.get_ref(), &session, &course).await {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Forbidden().body("This course is private to its organisation");
        }
        Err(response) => return response,
    }

    let user_id = match session.get::<i32>("user_id") {
        Ok(user_id) => user_id,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Session error: {}", err));
        }
    };

    let can_manage = if user_id.is_some() {
        match can_manage_course(db.get_ref(), &session, &course).await {
            Ok(can_manage) => can_manage,
            Err(response) if response.status() == actix_web::http::StatusCode::UNAUTHORIZED => {
                false
            }
            Err(response) => return response,
        }
    } else {
        false
    };

    let enrolled = match user_id {
        Some(user_id) => match enrollments::Entity::find()
            .filter(enrollments::Column::UserId.eq(user_id))
            .filter(enrollments::Column::CourseId.eq(course_id))
            .one(db.get_ref())
            .await
        {
            Ok(enrollment) => enrollment.is_some(),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error checking enrollment: {}", err));
            }
        },
        None => false,
    };

    let (module_rows, assignment_rows, quiz_rows) = tokio::join!(
        modules::Entity::find()
            .filter(modules::Column::CourseId.eq(course_id))
            .order_by_asc(modules::Column::Position)
            .all(db.get_ref()),
        assignments::Entity::find()
            .filter(assignments::Column::CourseId.eq(course_id))
            .all(db.get_ref()),
        quiz::Entity::find()
            .filter(quiz::Column::CourseId.eq(course_id))
            .all(db.get_ref()),
    );

    let module_rows = match module_rows {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding modules: {}", err));
        }
    };
    let assignment_rows = match assignment_rows {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding assignments: {}", err));
        }
    };
    let quiz_rows = match quiz_rows {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding quizzes: {}", err));
        }
    };

    let module_ids: Vec<i32> = module_rows.iter().map(|module| module.module_id).collect();
    let quiz_ids: Vec<i32> = quiz_rows.iter().map(|quiz| quiz.quiz_id).collect();

    let module_prerequisite_rows = if module_ids.is_empty() {
        Ok(Vec::new())
    } else {
        module_prerequisites::Entity::find()
            .filter(module_prerequisites::Column::ModuleId.is_in(module_ids.clone()))
            .order_by_asc(module_prerequisites::Column::PrerequisiteId)
            .all(db.get_ref())
            .await
    };
    let quiz_prerequisite_rows = if quiz_ids.is_empty() {
        Ok(Vec::new())
    } else {
        quiz_prerequisites::Entity::find()
            .filter(quiz_prerequisites::Column::QuizId.is_in(quiz_ids))
            .order_by_asc(quiz_prerequisites::Column::PrerequisiteId)
            .all(db.get_ref())
            .await
    };

    let mut module_prerequisites_by_id: HashMap<i32, Vec<i32>> = HashMap::new();
    match module_prerequisite_rows {
        Ok(rows) => {
            for row in rows {
                module_prerequisites_by_id
                    .entry(row.module_id)
                    .or_default()
                    .push(row.required_module_id);
            }
        }
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!(
                "Database error finding module prerequisites: {}",
                err
            ));
        }
    }

    let mut quiz_prerequisites_by_id: HashMap<i32, Vec<i32>> = HashMap::new();
    match quiz_prerequisite_rows {
        Ok(rows) => {
            for row in rows {
                quiz_prerequisites_by_id
                    .entry(row.quiz_id)
                    .or_default()
                    .push(row.required_module_id);
            }
        }
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!(
                "Database error finding quiz prerequisites: {}",
                err
            ));
        }
    }

    let completed_ids: HashSet<i32> = if enrolled && !module_ids.is_empty() {
        match module_progress::Entity::find()
            .filter(module_progress::Column::UserId.eq(user_id.unwrap_or_default()))
            .filter(module_progress::Column::ModuleId.is_in(module_ids.clone()))
            .filter(module_progress::Column::CompletedAt.is_not_null())
            .all(db.get_ref())
            .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(|progress| progress.module_id)
                .collect(),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding module progress: {}", err));
            }
        }
    } else {
        HashSet::new()
    };

    let modules: Vec<ModuleOverviewPayload> = module_rows
        .iter()
        .map(|module| ModuleOverviewPayload {
            module_id: module.module_id,
            course_id: module.course_id,
            title: module.title.clone(),
            position: module.position,
            prerequisite_module_ids: module_prerequisites_by_id
                .remove(&module.module_id)
                .unwrap_or_default(),
        })
        .collect();

    let quizzes: Vec<QuizOverviewPayload> = quiz_rows
        .into_iter()
        .map(|quiz| QuizOverviewPayload {
            quiz_id: quiz.quiz_id,
            course_id: quiz.course_id,
            title: quiz.title,
            description: quiz.description,
            max_attempts: quiz.max_attempts,
            time_limit: quiz.time_limit,
            starts_at: quiz.starts_at,
            ends_at: quiz.ends_at,
            created_at: quiz.created_at,
            prerequisite_module_ids: quiz_prerequisites_by_id
                .remove(&quiz.quiz_id)
                .unwrap_or_default(),
        })
        .collect();

    let total_modules = modules.len() as u64;
    let completed_modules = completed_ids.len() as u64;
    let course_progress = if enrolled {
        Some(CourseProgress {
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
        })
    } else {
        None
    };

    let module_progress = if enrolled {
        modules
            .iter()
            .map(|module| {
                let opened = completed_ids.contains(&module.module_id);

                CourseModuleProgress {
                    module_id: module.module_id,
                    opened,
                    progress_percent: if opened { 100 } else { 0 },
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    HttpResponse::Ok().json(CourseOverviewPayload {
        course,
        can_manage,
        enrolled,
        modules,
        assignments: assignment_rows,
        quizzes,
        course_progress,
        module_progress,
    })
}

#[get("/courses/search")]
pub async fn search_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    query: web::Query<CourseQuery>,
) -> impl Responder {
    let mut db_query = courses::Entity::find();

    if let Some(name) = &query.name {
        let pattern = format!("%{}%", name.trim());
        db_query = db_query.filter(
            Condition::any()
                .add(Expr::col(courses::Column::Name).ilike(pattern.clone()))
                .add(Expr::col(courses::Column::Description).ilike(pattern)),
        );
    }

    if let Some(instructor_id) = &query.instructor_id {
        db_query = db_query.filter(courses::Column::InstructorId.eq(*instructor_id))
    }

    if let Some(min_price) = query.min_price {
        db_query = db_query.filter(courses::Column::PriceCents.gte(min_price));
    }

    if let Some(max_price) = query.max_price {
        db_query = db_query.filter(courses::Column::PriceCents.lte(max_price));
    }
    if let Some(course_id) = query.course_id {
        db_query = db_query.filter(courses::Column::CourseId.eq(course_id))
    }

    if is_instructor_course_limited(&session) {
        let course_ids = match get_instructor_course_ids_for_session(db.get_ref(), &session).await {
            Ok(course_ids) => course_ids,
            Err(response) => return response,
        };

        if course_ids.is_empty() {
            return HttpResponse::Ok().json(Vec::<courses::Model>::new());
        }

        db_query = db_query.filter(courses::Column::CourseId.is_in(course_ids));
    } else if let Some(condition) = match accessible_course_condition(db.get_ref(), &session).await
    {
        Ok(condition) => condition,
        Err(response) => return response,
    } {
        db_query = db_query.filter(condition);
    }

    let result = db_query.all(db.get_ref()).await;

    match result {
        Ok(course) => {
            if course.is_empty() {
                HttpResponse::NotFound().body("No courses found")
            } else {
                HttpResponse::Ok().json(course)
            }
        }

        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[put("/courses/{course_id}")]
pub async fn update_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateCourse>,
) -> impl Responder {
    let course_id = path.into_inner();
    let data = body.into_inner();
    let existing = courses::Entity::find_by_id(course_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(course)) => {
            match can_manage_course(db.get_ref(), &session, &course).await {
                Ok(true) => {}
                Ok(false) => {
                    return HttpResponse::Forbidden()
                        .body("You can only update courses under your organisation");
                }
                Err(response) => return response,
            }

            let updated_price_cents = match data.price {
                Some(price) => match price_to_cents(price) {
                    Ok(price_cents) => price_cents,
                    Err(response) => return response,
                },
                None => course.price_cents.unwrap_or(0),
            };
            let updated_is_paid = data.is_paid.unwrap_or(course.is_paid.unwrap_or(false));

            if updated_is_paid && updated_price_cents <= 0 {
                return HttpResponse::BadRequest()
                    .body("Paid courses must have a price greater than zero");
            }

            let mut active: courses::ActiveModel = course.into();

            if let Some(name) = data.name {
                active.name = Set(Some(name));
            }
            if let Some(instructor_id) = data.instructor_id {
                active.instructor_id = Set(Some(instructor_id));
            }
            if let Some(org_id) = data.org_id {
                if has_role(&session, "LMS Admin") {
                    active.org_id = Set(Some(org_id));
                } else {
                    let user_org_id = match get_session_user_org_id(db.get_ref(), &session).await {
                        Ok(user_org_id) => user_org_id,
                        Err(response) => return response,
                    };

                    if user_org_id != Some(org_id) {
                        return HttpResponse::Forbidden().body(
                            "Organisation Admin cannot move courses outside their organisation",
                        );
                    }

                    active.org_id = Set(Some(org_id));
                }
            }
            if let Some(status) = data.status {
                let course_status = match status.as_str() {
                    "draft" => CourseStatus::Draft,
                    "published" => CourseStatus::Published,
                    "archived" => CourseStatus::Archived,

                    _ => {
                        return HttpResponse::BadRequest().body("Invalid course status");
                    }
                };

                active.status = Set(course_status);
            }

            if data.price.is_some() {
                active.price_cents = Set(Some(updated_price_cents));
            }
            if let Some(currency) = data.currency {
                active.currency = Set(Some(currency));
            }
            if let Some(is_paid) = data.is_paid {
                active.is_paid = Set(Some(is_paid));
            }

            if let Some(description) = data.description {
                active.description = Set(Some(description));
            }
            if let Some(background_image_url) = data.background_image_url {
                active.background_image_url = Set(Some(background_image_url));
            }
            if let Some(visibility) = data.visibility {
                active.visibility = Set(match normalize_course_visibility(Some(visibility)) {
                    Ok(visibility) => visibility,
                    Err(response) => return response,
                });
            }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body(format!("Course with id {} updated!", course_id)),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Update error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Course not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[post("/courses")]
pub async fn create_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateCourse>,
) -> impl Responder {
    let data = body.into_inner();

    if !has_role(&session, "Organisation Admin") {
        return HttpResponse::Forbidden()
            .body("Organisation Admin role required to create courses");
    }

    let session_user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return HttpResponse::Unauthorized().body("User not logged in"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Session error: {}", err));
        }
    };

    let org_id = match get_session_user_org_id(db.get_ref(), &session).await {
        Ok(Some(user_org_id)) => user_org_id,
        Ok(None) => {
            return HttpResponse::Forbidden()
                .body("Organisation Admin is not assigned to an organisation");
        }
        Err(response) => return response,
    };

    let price_cents = match price_to_cents(data.price) {
        Ok(price_cents) => price_cents,
        Err(response) => return response,
    };

    if data.is_paid && price_cents <= 0 {
        return HttpResponse::BadRequest().body("Paid courses must have a price greater than zero");
    }

    let visibility = match normalize_course_visibility(data.visibility) {
        Ok(visibility) => visibility,
        Err(response) => return response,
    };

    let course = courses::ActiveModel {
        name: Set(Some(data.name)),
        instructor_id: Set(Some(data.instructor_id.unwrap_or(session_user_id))),
        org_id: Set(Some(org_id)),

        status: Set(match data.status.as_str() {
            "draft" => CourseStatus::Draft,
            "published" => CourseStatus::Published,
            "archived" => CourseStatus::Archived,

            _ => {
                return HttpResponse::BadRequest().body("Invalid course status");
            }
        }),

        price_cents: Set(Some(price_cents)),
        currency: Set(Some(data.currency)),
        is_paid: Set(Some(data.is_paid)),
        description: Set(data.description),
        background_image_url: Set(data.background_image_url),
        visibility: Set(visibility),

        ..Default::default()
    };

    match course.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("New course created successfully!"),

        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

#[delete("/courses/{course_id}")]
pub async fn delete_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let course_id = path.into_inner();
    let existing = courses::Entity::find_by_id(course_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(course)) => {
            match can_manage_course(db.get_ref(), &session, &course).await {
                Ok(true) => {}
                Ok(false) => {
                    return HttpResponse::Forbidden()
                        .body("You can only delete courses under your organisation");
                }
                Err(response) => return response,
            }

            let active_model: courses::ActiveModel = course.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Course deleted!"),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Course not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
