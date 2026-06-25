use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, http::header, web};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::entity::{courses, enrollments, users};
use crate::services::auth_helpers::get_user_id;
use crate::services::certificate_service::{
    CertificateLinkPayload, certificate_payload, ensure_certificate_for_completion,
    generate_certificate_pdf_for_user,
    verify_certificate,
};
use crate::services::course_completion_service::{
    CourseCompletionStatus, load_completion_statuses,
};
use crate::services::course_service::can_manage_course;

#[derive(Serialize)]
struct MyCertificatePayload {
    course: courses::Model,
    status: CourseCompletionStatus,
    certificate: CertificateLinkPayload,
}

#[derive(Serialize)]
struct CourseCertificateRosterItem {
    user_id: i32,
    student_name: String,
    student_email: String,
    status: CourseCompletionStatus,
    certificate: CertificateLinkPayload,
}

#[get("/certificates/my")]
pub async fn get_my_certificates(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let user_id = match get_user_id(&session) {
        Ok(user_id) => user_id,
        Err(response) => return response,
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

    if enrollment_rows.is_empty() {
        return HttpResponse::Ok().json(Vec::<MyCertificatePayload>::new());
    }

    let completion_statuses = match load_completion_statuses(db.get_ref(), &enrollment_rows).await {
        Ok(statuses) => statuses,
        Err(response) => return response,
    };
    let course_ids: Vec<i32> = enrollment_rows
        .iter()
        .map(|enrollment| enrollment.course_id)
        .collect();
    let course_rows = match courses::Entity::find()
        .filter(courses::Column::CourseId.is_in(course_ids))
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding courses: {}", err));
        }
    };
    let mut courses_by_id: HashMap<i32, courses::Model> = course_rows
        .into_iter()
        .map(|course| (course.course_id, course))
        .collect();

    let mut payloads = Vec::new();
    for enrollment in &enrollment_rows {
        let Some(status) = completion_statuses
            .get(&(enrollment.user_id, enrollment.course_id))
            .filter(|status| status.completed)
        else {
            continue;
        };
        let Some(course) = courses_by_id.remove(&enrollment.course_id) else {
            continue;
        };
        let certificate =
            match ensure_certificate_for_completion(db.get_ref(), enrollment, status).await {
                Ok(certificate) => certificate,
                Err(response) => return response,
            };

        payloads.push(MyCertificatePayload {
            course,
            status: status.clone(),
            certificate: certificate_payload(&certificate),
        });
    }

    HttpResponse::Ok().json(payloads)
}

#[get("/courses/{course_id}/certificates")]
pub async fn get_course_certificates(
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

    match can_manage_course(db.get_ref(), &session, &course).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().body("You cannot manage this course"),
        Err(response) => return response,
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
        return HttpResponse::Ok().json(Vec::<CourseCertificateRosterItem>::new());
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

    let mut roster = Vec::new();
    for enrollment in &enrollment_rows {
        let Some(status) = completion_statuses
            .get(&(enrollment.user_id, enrollment.course_id))
            .filter(|status| status.completed)
        else {
            continue;
        };
        let Some(user) = users_by_id.get(&enrollment.user_id) else {
            continue;
        };
        let certificate =
            match ensure_certificate_for_completion(db.get_ref(), enrollment, status).await {
                Ok(certificate) => certificate,
                Err(response) => return response,
            };

        roster.push(CourseCertificateRosterItem {
            user_id: enrollment.user_id,
            student_name: format!("{} {}", user.first_name, user.last_name)
                .trim()
                .to_string(),
            student_email: user.email.clone(),
            status: status.clone(),
            certificate: certificate_payload(&certificate),
        });
    }

    roster.sort_by(|a, b| {
        a.student_name
            .to_lowercase()
            .cmp(&b.student_name.to_lowercase())
            .then_with(|| a.student_email.cmp(&b.student_email))
    });

    HttpResponse::Ok().json(roster)
}

#[get("/certificates/{certificate_id}/download")]
pub async fn download_my_certificate(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let user_id = match get_user_id(&session) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };

    match generate_certificate_pdf_for_user(db.get_ref(), path.into_inner(), user_id).await {
        Ok(Some(pdf)) => HttpResponse::Ok()
            .insert_header((header::CONTENT_TYPE, "application/pdf"))
            .insert_header((
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", pdf.filename),
            ))
            .body(pdf.bytes),
        Ok(None) => HttpResponse::NotFound().body("Certificate not found"),
        Err(response) => response,
    }
}

#[get("/certificates/verify/{token}")]
pub async fn verify_certificate_token(
    db: web::Data<DatabaseConnection>,
    path: web::Path<String>,
) -> impl Responder {
    let token = match Uuid::parse_str(&path.into_inner()) {
        Ok(token) => token,
        Err(_) => return HttpResponse::NotFound().body("Certificate not found"),
    };

    match verify_certificate(db.get_ref(), token).await {
        Ok(Some(payload)) => HttpResponse::Ok().json(payload),
        Ok(None) => HttpResponse::NotFound().body("Certificate not found"),
        Err(response) => response,
    }
}
