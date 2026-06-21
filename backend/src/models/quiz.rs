use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::entity::quiz_questions::QuestionType;

#[derive(Serialize, Deserialize)]
pub struct SaveQuizOption {
    pub option_text: String,
    pub is_correct: bool,
    pub position: i32,
}

#[derive(Serialize, Deserialize)]
pub struct SaveQuizQuestion {
    pub question_type: QuestionType,
    pub question_text: String,
    pub position: i32,
    pub points: i32,
    pub options: Vec<SaveQuizOption>,
}

#[derive(Serialize, Deserialize)]
pub struct SaveQuizDraft {
    pub course_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub max_attempts: Option<i32>,
    pub time_limit: Option<i32>,
    pub starts_at: Option<NaiveDateTime>,
    pub prerequisite_module_ids: Vec<i32>,
    pub questions: Vec<SaveQuizQuestion>,
}

#[derive(Serialize, Deserialize)]
pub struct Quiz {
    pub quiz_id: i32,
    pub course_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub max_attempts: Option<i32>,
    pub time_limit: Option<i32>,
    pub starts_at: Option<NaiveDateTime>,
    pub ends_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateQuiz {
    pub course_id: Option<i32>,
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub max_attempts: Option<Option<i32>>,
    #[serde(default)]
    pub time_limit: Option<Option<i32>>,
    #[serde(default)]
    pub starts_at: Option<Option<NaiveDateTime>>,
    pub prerequisite_module_ids: Option<Vec<i32>>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateQuiz {
    pub course_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub max_attempts: Option<i32>,
    pub time_limit: Option<i32>,
    pub starts_at: Option<NaiveDateTime>,
    pub prerequisite_module_ids: Option<Vec<i32>>,
}
