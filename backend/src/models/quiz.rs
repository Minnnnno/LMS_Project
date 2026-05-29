use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

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
    pub description: Option<String>,
    pub max_attempts: Option<i32>,
    pub time_limit: Option<i32>,
    pub starts_at: Option<NaiveDateTime>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateQuiz {
    pub course_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub max_attempts: Option<i32>,
    pub time_limit: Option<i32>,
    pub starts_at: Option<NaiveDateTime>,
}