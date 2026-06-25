use actix_web::HttpResponse;
use chrono::Utc;
use qrcode::{Color as QrColor, QrCode};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use serde::Serialize;
use uuid::Uuid;

use crate::entity::{course_certificates, courses, enrollments, organisations, users};
use crate::services::course_completion_service::{
    CourseCompletionStatus, load_completion_statuses,
};

const PDF_PAGE_WIDTH: f32 = 842.0;
const PDF_PAGE_HEIGHT: f32 = 595.0;

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

pub struct CertificatePdf {
    pub filename: String,
    pub bytes: Vec<u8>,
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

fn pdf_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\\' => "\\\\".to_string(),
            '(' => "\\(".to_string(),
            ')' => "\\)".to_string(),
            '\n' | '\r' | '\t' => " ".to_string(),
            ch if ch.is_ascii() && !ch.is_control() => ch.to_string(),
            _ => "?".to_string(),
        })
        .collect()
}

fn filename_part(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch
            } else if ch.is_whitespace() || matches!(ch, '-' | '_') {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|ch| *ch != '\0')
        .collect();
    let collapsed = cleaned
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if collapsed.is_empty() {
        "certificate".to_string()
    } else {
        collapsed
    }
}

fn pdf_text_width(text: &str, size: f32) -> f32 {
    let units: f32 = text
        .chars()
        .map(|ch| match ch {
            ' ' => 278.0,
            'i' | 'j' | 'l' | 'I' => 278.0,
            'f' | 'r' | 't' => 333.0,
            'm' | 'w' | 'M' | 'W' => 833.0,
            'A' | 'B' | 'C' | 'D' | 'G' | 'H' | 'N' | 'O' | 'Q' | 'R' | 'U' | 'V' | 'X'
            | 'Y' => 722.0,
            'E' | 'F' | 'L' | 'P' | 'S' | 'T' | 'Z' => 667.0,
            'J' => 389.0,
            'K' => 722.0,
            'a' | 'b' | 'c' | 'd' | 'e' | 'g' | 'h' | 'k' | 'n' | 'o' | 'p' | 'q' | 's'
            | 'u' | 'v' | 'x' | 'y' | 'z' => 556.0,
            '0'..='9' => 556.0,
            _ => 500.0,
        })
        .sum();
    units * size / 1000.0
}

fn text_at(content: &mut String, text: &str, x: f32, y: f32, size: f32, font: &str) {
    content.push_str(&format!(
        "BT /{} {} Tf {} {} Td ({}) Tj ET\n",
        font,
        size,
        x,
        y,
        pdf_text(text)
    ));
}

fn centered_text(content: &mut String, text: &str, y: f32, size: f32, font: &str) {
    let estimated_width = pdf_text_width(text, size);
    let x = (PDF_PAGE_WIDTH - estimated_width).max(0.0) / 2.0;
    text_at(content, text, x, y, size, font);
}

fn centered_text_at(content: &mut String, text: &str, center_x: f32, y: f32, size: f32, font: &str) {
    let estimated_width = pdf_text_width(text, size);
    let x = (center_x - estimated_width / 2.0).max(0.0);
    text_at(content, text, x, y, size, font);
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let separator = if current.is_empty() { 0 } else { 1 };
        if !current.is_empty() && current.len() + separator + word.len() > max_chars {
            lines.push(current);
            current = String::new();
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn centered_wrapped_text(
    content: &mut String,
    text: &str,
    start_y: f32,
    size: f32,
    font: &str,
    max_chars: usize,
) {
    for (index, line) in wrap_text(text, max_chars).iter().enumerate() {
        centered_text(content, line, start_y - index as f32 * (size + 7.0), size, font);
    }
}

fn draw_qr_code(content: &mut String, url: &str, x: f32, y: f32, size: f32) -> Result<(), String> {
    let code = QrCode::new(url.as_bytes()).map_err(|err| err.to_string())?;
    let width = code.width();
    let quiet_zone = 4usize;
    let total_width = width + quiet_zone * 2;
    let module = size / total_width as f32;

    content.push_str("0 0 0 rg\n");
    for row in 0..width {
        for col in 0..width {
            if code[(col, row)] == QrColor::Dark {
                let px = x + (col + quiet_zone) as f32 * module;
                let py = y + (total_width - row - quiet_zone - 1) as f32 * module;
                content.push_str(&format!("{} {} {} {} re f\n", px, py, module, module));
            }
        }
    }

    Ok(())
}

fn build_pdf(content_stream: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /Contents 4 0 R >>",
            PDF_PAGE_WIDTH, PDF_PAGE_HEIGHT
        ),
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content_stream.as_bytes().len(),
            content_stream
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".to_string(),
    ];

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());

    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }

    let xref_start = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_start
        )
        .as_bytes(),
    );

    pdf
}

fn certificate_pdf_bytes(
    student_name: &str,
    course_name: &str,
    organisation_name: Option<&str>,
    issued_date: &str,
    verification_url: &str,
) -> Result<Vec<u8>, String> {
    let mut content = String::new();

    content.push_str("0.94 0.94 0.93 rg 0 0 842 595 re f\n");
    content.push_str("1 1 1 rg 46 42 750 511 re f\n");

    content.push_str("0 0 0 RG 1.4 w 46 42 750 511 re S\n");

    content.push_str("0 0 0 rg\n");
    content.push_str("64 531 72 4 re f\n");
    content.push_str("64 463 4 72 re f\n");
    content.push_str("706 531 72 4 re f\n");
    content.push_str("774 463 4 72 re f\n");
    content.push_str("64 60 72 4 re f\n");
    content.push_str("64 60 4 72 re f\n");
    content.push_str("706 60 72 4 re f\n");
    content.push_str("774 60 4 72 re f\n");

    centered_text(&mut content, "SkillUp LMS", 502.0, 15.0, "F2");
    if let Some(name) = organisation_name
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        centered_wrapped_text(&mut content, name, 478.0, 10.0, "F1", 72);
    }
    content.push_str("0.55 0.55 0.55 RG 0.8 w 320 462 m 522 462 l S\n");

    content.push_str("0 0 0 rg\n");
    centered_text(&mut content, "Certificate of Completion", 407.0, 31.0, "F2");
    content.push_str("0 0 0 RG 1.1 w 283 386 m 559 386 l S\n");

    content.push_str("0 0 0 rg\n");
    centered_text(&mut content, "This is to certify that", 345.0, 12.0, "F1");
    centered_wrapped_text(&mut content, student_name, 294.0, 34.0, "F2", 44);
    content.push_str("0.70 0.70 0.70 RG 0.7 w 260 271 m 582 271 l S\n");

    content.push_str("0 0 0 rg\n");
    centered_text(&mut content, "has completed", 238.0, 13.0, "F1");
    centered_wrapped_text(&mut content, course_name, 194.0, 24.0, "F2", 52);

    content.push_str("0 0 0 rg\n");
    text_at(&mut content, "Issued on", 126.0, 116.0, 10.0, "F1");
    text_at(&mut content, issued_date, 126.0, 92.0, 14.0, "F2");
    content.push_str("0 0 0 RG 0.8 w 126 84 m 284 84 l S\n");

    content.push_str("0 0 0 rg\n");
    centered_text_at(&mut content, "Verification", 668.0, 160.0, 10.0, "F1");
    draw_qr_code(&mut content, verification_url, 632.0, 74.0, 72.0)?;
    content.push_str("0 0 0 RG 0.8 w 632 74 72 72 re S\n");

    Ok(build_pdf(&content))
}

pub async fn generate_certificate_pdf_for_user(
    db: &DatabaseConnection,
    certificate_id: i32,
    user_id: i32,
) -> Result<Option<CertificatePdf>, HttpResponse> {
    let Some(certificate) = course_certificates::Entity::find_by_id(certificate_id)
        .filter(course_certificates::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding certificate: {}", err))
        })?
    else {
        return Ok(None);
    };

    let Some(user) = users::Entity::find_by_id(certificate.user_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding user: {}", err))
        })?
    else {
        return Err(HttpResponse::NotFound().body("Certificate owner not found"));
    };

    let Some(course) = courses::Entity::find_by_id(certificate.course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
    else {
        return Err(HttpResponse::NotFound().body("Certificate course not found"));
    };

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

    if certificate.revoked_at.is_some() || !current_completion {
        return Err(HttpResponse::Forbidden().body("Certificate is not currently valid"));
    }

    let student_name = format!("{} {}", user.first_name, user.last_name)
        .trim()
        .to_string();
    let course_name = course
        .name
        .clone()
        .unwrap_or_else(|| "Untitled course".to_string());
    let organisation_name = if let Some(org_id) = course.org_id {
        organisations::Entity::find_by_id(org_id)
            .one(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError()
                    .body(format!("Database error finding organisation: {}", err))
            })?
            .map(|organisation| organisation.org_name)
    } else {
        None
    };
    let issued_date = certificate.issued_at.format("%d %B %Y").to_string();
    let verification_url = certificate_url(certificate.verification_token);
    let bytes = certificate_pdf_bytes(
        &student_name,
        &course_name,
        organisation_name.as_deref(),
        &issued_date,
        &verification_url,
    )
    .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Certificate PDF generation error: {}", err))
        })?;
    let filename = format!(
        "SkillUp-Certificate-{}.pdf",
        filename_part(&course_name)
    );

    Ok(Some(CertificatePdf { filename, bytes }))
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
