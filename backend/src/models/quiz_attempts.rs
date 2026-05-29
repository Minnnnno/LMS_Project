use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct QuizAttempt {
    pub attempt_id: i32,
    pub quiz_id: i32,
    pub user_id: i32,
    pub started_at: NaiveDateTime,
    pub submitted_at: Option<NaiveDateTime>,
    pub total_score: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct SubmitAttempt {
    pub submitted_at: Option<NaiveDateTime>,
}

#[derive(Serialize, Deserialize)]
pub struct MarkAttempt {
    pub total_score: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateAttempt {
    pub quiz_id: i32,
    pub user_id: i32,
}