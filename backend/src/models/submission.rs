use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateSubmission {
    pub submission_text: Option<String>,
    pub file_url: Option<String>,
    pub cloudinary_public_id: Option<String>,
    pub file_name: Option<String>,
    pub file_content_type: Option<String>,
    pub file_size: Option<i64>,
}

#[derive(Serialize)]
pub struct StudentSubmission {
    pub submission_id: i32,
    pub assignment_id: i32,
    pub submitted_at: NaiveDateTime,
    pub submission_text: Option<String>,
    pub file_url: Option<String>,
    pub cloudinary_public_id: Option<String>,
    pub score: Option<Decimal>,
    pub feedback: Option<String>,
}

#[derive(Serialize)]
pub struct StaffSubmission {
    pub submission_id: i32,
    pub assignment_id: i32,
    pub user_id: i32,
    pub student_name: String,
    pub student_email: String,
    pub submitted_at: NaiveDateTime,
    pub submission_text: Option<String>,
    pub file_url: Option<String>,
    pub cloudinary_public_id: Option<String>,
    pub score: Option<Decimal>,
    pub feedback: Option<String>,
    pub is_latest: bool,
}

#[derive(Deserialize)]
pub struct GradeSubmission {
    pub score: Decimal,
    pub feedback: Option<String>,
}
