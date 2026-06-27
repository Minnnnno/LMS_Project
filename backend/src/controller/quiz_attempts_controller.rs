use crate::services::{quiz_attempt_service, quiz_review_service};
use actix_session::Session;
use actix_web::{Responder, delete, get, post, put, web};
use sea_orm::DatabaseConnection;

// Staff only: see all attempts for a quiz
#[get("/quiz-attempts/quiz/{quiz_id}")]
pub async fn get_attempts_by_quiz_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_review_service::list_staff_attempts(db.get_ref(), &session, path.into_inner()).await
}

// Students see their own graded attempt review
#[get("/quiz-attempts/my/{attempt_id}/review")]
pub async fn get_my_attempt_review(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_review_service::get_student_review(db.get_ref(), &session, path.into_inner()).await
}

// Students see their own attempt availability for all quizzes in a course
#[get("/quiz-attempts/my/course/{course_id}/status")]
pub async fn get_my_attempt_statuses_by_course(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::list_my_attempt_statuses_by_course(
        db.get_ref(),
        &session,
        path.into_inner(),
    )
    .await
}

// Students create their own attempts; user_id is pulled from session
#[post("/quiz/{quiz_id}/attempt")]
pub async fn create_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::create_attempt(db.get_ref(), &session, path.into_inner()).await
}

// Students can only submit their own attempt
#[put("/quiz-attempts/{attempt_id}/submit")]
pub async fn submit_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::submit_attempt(db.get_ref(), &session, path.into_inner()).await
}

// Staff can delete attempts for quizzes they can manage
#[delete("/quiz-attempts/{attempt_id}")]
pub async fn delete_quiz_attempt(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    quiz_attempt_service::delete_attempt(db.get_ref(), &session, path.into_inner()).await
}
