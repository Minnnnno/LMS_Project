use actix_web::HttpResponse;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use serde::Serialize;
use uuid::Uuid;

use crate::entity::{course_certificates, courses, enrollments, users};
use crate::services::course_completion_service::{
    CourseCompletionStatus, load_completion_statuses,
};

#[derive(Clone, Serialize)]
pub struct CertificateLinkPayload {
    pub certificate_id: i32,
    pub user_id: i32,
    pub course_id: i32,
    pub verification_token: Uuid,
    pub verification_url: String,
    pub issued_at: chrono::DateTime<chrono::Utc>,
    pub completion_source: String,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
pub struct CertificateVerificationPayload {
    pub valid: bool,
    pub status: String,
    pub student_name: String,
    pub course_name: String,
    pub issued_at: chrono::DateTime<chrono::Utc>,
    pub completion_source: String,
    pub verification_token: Uuid,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn certificate_url(token: Uuid) -> String {
    let base_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    format!(
        "{}/verify/certificate/{}",
        base_url.trim_end_matches('/'),
        token
    )
}

pub fn certificate_payload(certificate: &course_certificates::Model) -> CertificateLinkPayload {
    CertificateLinkPayload {
        certificate_id: certificate.certificate_id,
        user_id: certificate.user_id,
        course_id: certificate.course_id,
        verification_token: certificate.verification_token,
        verification_url: certificate_url(certificate.verification_token),
        issued_at: certificate.issued_at,
        completion_source: certificate.completion_source.clone(),
        revoked_at: certificate.revoked_at,
    }
}

pub async fn ensure_certificate_for_completion(
    db: &DatabaseConnection,
    enrollment: &enrollments::Model,
    status: &CourseCompletionStatus,
) -> Result<course_certificates::Model, HttpResponse> {
    if !status.completed {
        return Err(HttpResponse::BadRequest().body("Course is not completed"));
    }

    let existing = course_certificates::Entity::find()
        .filter(course_certificates::Column::UserId.eq(enrollment.user_id))
        .filter(course_certificates::Column::CourseId.eq(enrollment.course_id))
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding certificate: {}", err))
        })?;

    if let Some(certificate) = existing {
        if certificate.revoked_at.is_some()
            || certificate.completion_source != status.completion_source
        {
            let mut active = certificate.into_active_model();
            active.revoked_at = Set(None);
            active.completion_source = Set(status.completion_source.clone());
            active.updated_at = Set(Some(Utc::now()));
            return active.update(db).await.map_err(|err| {
                HttpResponse::InternalServerError()
                    .body(format!("Database error updating certificate: {}", err))
            });
        }

        return Ok(certificate);
    }

    let now = Utc::now();
    let certificate = course_certificates::ActiveModel {
        user_id: Set(enrollment.user_id),
        course_id: Set(enrollment.course_id),
        verification_token: Set(Uuid::new_v4()),
        issued_at: Set(now),
        completion_source: Set(status.completion_source.clone()),
        revoked_at: Set(None),
        created_at: Set(now),
        updated_at: Set(None),
        ..Default::default()
    };

    certificate.insert(db).await.map_err(|err| {
        HttpResponse::InternalServerError()
            .body(format!("Database error creating certificate: {}", err))
    })
}

pub async fn revoke_certificate_if_incomplete(
    db: &DatabaseConnection,
    enrollment: &enrollments::Model,
) -> Result<(), HttpResponse> {
    let statuses = load_completion_statuses(db, std::slice::from_ref(enrollment)).await?;
    let completed = statuses
        .get(&(enrollment.user_id, enrollment.course_id))
        .is_some_and(|status| status.completed);

    if completed {
        return Ok(());
    }

    let Some(certificate) = course_certificates::Entity::find()
        .filter(course_certificates::Column::UserId.eq(enrollment.user_id))
        .filter(course_certificates::Column::CourseId.eq(enrollment.course_id))
        .filter(course_certificates::Column::RevokedAt.is_null())
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding certificate: {}", err))
        })?
    else {
        return Ok(());
    };

    let now = Utc::now();
    let mut active = certificate.into_active_model();
    active.revoked_at = Set(Some(now));
    active.updated_at = Set(Some(now));
    active.update(db).await.map_err(|err| {
        HttpResponse::InternalServerError()
            .body(format!("Database error revoking certificate: {}", err))
    })?;

    Ok(())
}

pub async fn verify_certificate(
    db: &DatabaseConnection,
    token: Uuid,
) -> Result<Option<CertificateVerificationPayload>, HttpResponse> {
    let Some(certificate) = course_certificates::Entity::find()
        .filter(course_certificates::Column::VerificationToken.eq(token))
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding certificate: {}", err))
        })?
    else {
        return Ok(None);
    };

    let user = users::Entity::find_by_id(certificate.user_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding user: {}", err))
        })?;
    let course = courses::Entity::find_by_id(certificate.course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?;
    let enrollment = enrollments::Entity::find_by_id((certificate.user_id, certificate.course_id))
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding enrollment: {}", err))
        })?;

    let current_completion = if let Some(enrollment) = enrollment.as_ref() {
        let statuses = load_completion_statuses(db, std::slice::from_ref(enrollment)).await?;
        statuses
            .get(&(certificate.user_id, certificate.course_id))
            .is_some_and(|status| status.completed)
    } else {
        false
    };
    let valid = certificate.revoked_at.is_none()
        && current_completion
        && user.is_some()
        && course.is_some();
    let student_name = user
        .map(|user| {
            format!("{} {}", user.first_name, user.last_name)
                .trim()
                .to_string()
        })
        .unwrap_or_else(|| "Unknown student".to_string());
    let course_name = course
        .and_then(|course| course.name)
        .unwrap_or_else(|| "Unknown course".to_string());

    Ok(Some(CertificateVerificationPayload {
        valid,
        status: if valid { "valid" } else { "invalid" }.to_string(),
        student_name,
        course_name,
        issued_at: certificate.issued_at,
        completion_source: certificate.completion_source,
        verification_token: certificate.verification_token,
        revoked_at: certificate.revoked_at,
    }))
}
