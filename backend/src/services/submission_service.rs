use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::entity::{assignments, submissions, users};
use crate::models::submission::{CreateSubmission, StudentSubmission};
use crate::services::auth_helpers::{get_user_id, is_enrolled};
use crate::services::mailer_service::{send_mail_message, MailRequest};

async fn require_enrolled_for_assignment(
    db: &DatabaseConnection,
    user_id: i32,
    assignment_id: i32,
) -> Result<assignments::Model, HttpResponse> {
    let assignment = assignments::Entity::find_by_id(assignment_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding assignment: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Assignment not found"))?;

    match is_enrolled(db, user_id, assignment.course_id).await {
        Ok(true) => Ok(assignment),
        Ok(false) => Err(HttpResponse::Forbidden()
            .body("You must be enrolled in this course to submit assignments")),
        Err(response) => Err(response),
    }
}

fn to_student_submission(submission: submissions::Model) -> StudentSubmission {
    StudentSubmission {
        submission_id: submission.submission_id,
        assignment_id: submission.assignment_id,
        submitted_at: submission.submitted_at,
        submission_text: submission.submission_text,
        file_url: submission.file_url,
        cloudinary_public_id: submission.cloudinary_public_id,
        score: submission.score,
        feedback: submission.feedback,
    }
}

fn get_file_extension(file_name: &str) -> Option<String> {
    file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension.trim().to_ascii_lowercase())
        .filter(|extension| !extension.is_empty())
}

fn content_type_matches(expected_file_type: &str, content_type: &str) -> bool {
    let content_type = content_type.to_ascii_lowercase();

    match expected_file_type {
        "pdf" => content_type == "application/pdf",
        "docx" => {
            content_type
                == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        "pptx" => {
            content_type
                == "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        }
        "xlsx" => {
            content_type == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        }
        "zip" => content_type == "application/zip" || content_type == "application/x-zip-compressed",
        "image" => content_type.starts_with("image/"),
        _ => true,
    }
}

fn extension_matches(expected_file_type: &str, extension: &str) -> bool {
    match expected_file_type {
        "pdf" => extension == "pdf",
        "docx" => extension == "docx",
        "pptx" => extension == "pptx",
        "xlsx" => extension == "xlsx",
        "zip" => extension == "zip",
        "image" => ["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg"].contains(&extension),
        _ => true,
    }
}

fn validate_submission_file(
    assignment: &assignments::Model,
    data: &CreateSubmission,
) -> Result<(), HttpResponse> {
    if data.file_url.is_none() {
        return Ok(());
    }

    if let Some(max_file_size_mb) = assignment.max_file_size_mb {
        if let Some(file_size) = data.file_size {
            let max_bytes = i64::from(max_file_size_mb) * 1024 * 1024;

            if file_size > max_bytes {
                return Err(HttpResponse::BadRequest()
                    .body(format!("File must be {} MB or smaller", max_file_size_mb)));
            }
        } else {
            return Err(HttpResponse::BadRequest().body("File size is required"));
        }
    }

    let expected_file_type = match assignment.expected_file_type.as_deref() {
        Some(value) if !value.trim().is_empty() => value,
        _ => return Ok(()),
    };

    let file_name = data
        .file_name
        .as_deref()
        .ok_or_else(|| HttpResponse::BadRequest().body("File name is required"))?;
    let extension = get_file_extension(file_name)
        .ok_or_else(|| HttpResponse::BadRequest().body("File extension is required"))?;

    if !extension_matches(expected_file_type, &extension) {
        return Err(HttpResponse::BadRequest().body(format!(
            "File type must match the expected {} format",
            expected_file_type
        )));
    }

    if let Some(content_type) = data.file_content_type.as_deref() {
        if !content_type_matches(expected_file_type, content_type) {
            return Err(HttpResponse::BadRequest().body(format!(
                "File content type must match the expected {} format",
                expected_file_type
            )));
        }
    }

    Ok(())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn build_submission_confirmation_email(
    user: &users::Model,
    assignment: &assignments::Model,
    submission: &submissions::Model,
    file_name: Option<&str>,
) -> String {
    let student_name = escape_html(&format!("{} {}", user.first_name, user.last_name));
    let assignment_title = escape_html(&assignment.title);
    let submitted_at = escape_html(&submission.submitted_at.format("%d %b %Y, %I:%M %p").to_string());
    let file_label = escape_html(file_name.unwrap_or("Uploaded file"));
    let file_link = submission
        .file_url
        .as_deref()
        .map(|url| {
            format!(
                r#"<a href="{url}" style="display:inline-block;margin-top:10px;padding:10px 14px;border-radius:8px;background:#171717;color:#ffffff;text-decoration:none;font-weight:700;">Open submitted file</a>"#,
                url = escape_html(url)
            )
        })
        .unwrap_or_default();

    format!(
        r#"<!doctype html>
<html>
<body style="margin:0;padding:0;background:#f4f6f8;font-family:Arial,Helvetica,sans-serif;color:#1f2937;">
  <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="background:#f4f6f8;padding:28px 12px;">
    <tr>
      <td align="center">
        <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="max-width:620px;background:#ffffff;border:1px solid #e5e7eb;border-radius:12px;overflow:hidden;">
          <tr>
            <td style="padding:24px 28px;background:#171717;color:#ffffff;">
              <div style="font-size:13px;font-weight:700;letter-spacing:0;text-transform:uppercase;color:#cbd5e1;">SkillUp LMS</div>
              <h1 style="margin:8px 0 0;font-size:24px;line-height:1.25;">Assignment submitted</h1>
            </td>
          </tr>
          <tr>
            <td style="padding:28px;">
              <p style="margin:0 0 16px;font-size:16px;line-height:1.55;">Hi {student_name},</p>
              <p style="margin:0 0 18px;font-size:16px;line-height:1.55;">You have just submitted a file for <strong>{assignment_title}</strong>.</p>
              <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="margin:18px 0;border:1px solid #e5e7eb;border-radius:10px;background:#f9fafb;">
                <tr>
                  <td style="padding:14px 16px;color:#64748b;font-size:13px;font-weight:700;width:130px;">Assignment</td>
                  <td style="padding:14px 16px;font-size:14px;">{assignment_title}</td>
                </tr>
                <tr>
                  <td style="padding:14px 16px;color:#64748b;font-size:13px;font-weight:700;border-top:1px solid #e5e7eb;">File</td>
                  <td style="padding:14px 16px;font-size:14px;border-top:1px solid #e5e7eb;">{file_label}</td>
                </tr>
                <tr>
                  <td style="padding:14px 16px;color:#64748b;font-size:13px;font-weight:700;border-top:1px solid #e5e7eb;">Submitted</td>
                  <td style="padding:14px 16px;font-size:14px;border-top:1px solid #e5e7eb;">{submitted_at}</td>
                </tr>
              </table>
              {file_link}
              <p style="margin:22px 0 0;font-size:14px;line-height:1.5;color:#64748b;">Keep this email as your submission confirmation.</p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"#
    )
}

async fn send_submission_confirmation_email(
    db: &DatabaseConnection,
    user_id: i32,
    assignment: &assignments::Model,
    submission: &submissions::Model,
    file_name: Option<&str>,
) -> Result<(), String> {
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|err| format!("Database error finding user for email: {}", err))?
        .ok_or_else(|| "User not found for submission email".to_string())?;

    send_mail_message(MailRequest {
        to: user.email.clone(),
        subject: format!("Submission received: {}", assignment.title),
        body: build_submission_confirmation_email(&user, assignment, submission, file_name),
        is_html: true,
    })
}

pub async fn create_submission(
    db: &DatabaseConnection,
    session: &Session,
    assignment_id: i32,
    data: CreateSubmission,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let assignment = match require_enrolled_for_assignment(db, user_id, assignment_id).await {
        Ok(assignment) => assignment,
        Err(response) => return response,
    };

    if assignment.allow_file_submission == Some(false) && data.file_url.is_some() {
        return HttpResponse::BadRequest().body("This assignment does not accept file submissions");
    }

    if let Err(response) = validate_submission_file(&assignment, &data) {
        return response;
    }

    if assignment.allow_text_submission == Some(false) && data.submission_text.is_some() {
        return HttpResponse::BadRequest().body("This assignment does not accept text submissions");
    }

    let submission_text = data
        .submission_text
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());

    if submission_text.is_none() && data.file_url.is_none() {
        return HttpResponse::BadRequest().body("Please attach a file or enter submission text");
    }

    let submission = submissions::ActiveModel {
        assignment_id: Set(assignment_id),
        user_id: Set(user_id),
        submission_text: Set(submission_text),
        file_url: Set(data.file_url),
        cloudinary_public_id: Set(data.cloudinary_public_id),
        ..Default::default()
    };

    match submission.insert(db).await {
        Ok(saved) => {
            if let Err(err) = send_submission_confirmation_email(
                db,
                user_id,
                &assignment,
                &saved,
                data.file_name.as_deref(),
            )
            .await
            {
                eprintln!("Submission confirmation email error: {}", err);
            }

            HttpResponse::Ok().json(to_student_submission(saved))
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Database error saving submission: {}", err))
        }
    }
}

pub async fn list_my_submissions(
    db: &DatabaseConnection,
    session: &Session,
    assignment_id: i32,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    if let Err(response) = require_enrolled_for_assignment(db, user_id, assignment_id).await {
        return response;
    }

    match submissions::Entity::find()
        .filter(submissions::Column::AssignmentId.eq(assignment_id))
        .filter(submissions::Column::UserId.eq(user_id))
        .order_by_desc(submissions::Column::SubmittedAt)
        .all(db)
        .await
    {
        Ok(submissions) => HttpResponse::Ok().json(
            submissions
                .into_iter()
                .map(to_student_submission)
                .collect::<Vec<_>>(),
        ),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Database error finding submission: {}", err))
        }
    }
}
