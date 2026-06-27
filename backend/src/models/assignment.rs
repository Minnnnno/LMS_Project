use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Assignment {
    pub assignment_id: i32,
    pub course_id: i32,
    pub module_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<NaiveDateTime>,
    pub max_score: Option<Decimal>,
    pub passing_mark: Decimal,
    pub assignment_brief_url: Option<String>,
    pub expected_file_type: Option<String>,
    pub allow_text_submission: Option<bool>,
    pub allow_file_submission: Option<bool>,
    pub max_file_size_mb: Option<i32>,
    pub submission_instructions: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AssignmentResponse {
    pub assignment_id: i32,
    pub course_id: i32,
    pub module_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<NaiveDateTime>,
    pub max_score: Option<Decimal>,
    pub passing_mark: Decimal,
    pub assignment_brief_url: Option<String>,
    pub expected_file_type: Option<String>,
    pub allow_text_submission: Option<bool>,
    pub allow_file_submission: Option<bool>,
    pub max_file_size_mb: Option<i32>,
    pub submission_instructions: Option<String>,
    pub prerequisite_module_ids: Vec<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateAssignment {
    pub course_id: i32,
    pub module_id: i32,
    pub title: String,
    pub description: String,
    pub due_date: NaiveDateTime,
    pub max_score: Decimal,
    pub passing_mark: Option<Decimal>,
    pub assignment_brief_url: Option<String>,
    pub expected_file_type: Option<String>,
    pub allow_text_submission: Option<bool>,
    pub allow_file_submission: Option<bool>,
    pub max_file_size_mb: Option<i32>,
    pub submission_instructions: Option<String>,
    #[serde(default)]
    pub prerequisite_module_ids: Vec<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateAssignment {
    pub course_id: Option<i32>,
    pub module_id: Option<i32>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub due_date: Option<NaiveDateTime>,
    pub max_score: Option<Decimal>,
    pub passing_mark: Option<Decimal>,
    #[serde(default)]
    pub assignment_brief_url: Option<Option<String>>,
    #[serde(default)]
    pub expected_file_type: Option<Option<String>>,
    pub allow_text_submission: Option<bool>,
    pub allow_file_submission: Option<bool>,
    #[serde(default)]
    pub max_file_size_mb: Option<Option<i32>>,
    #[serde(default)]
    pub submission_instructions: Option<Option<String>>,
    #[serde(default)]
    pub prerequisite_module_ids: Option<Vec<i32>>,
}
