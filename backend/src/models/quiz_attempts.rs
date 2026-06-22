use chrono::NaiveDateTime;
use serde::Serialize;

use crate::entity::{quiz, quiz_attempts, quiz_questions::QuestionType};
use crate::models::quiz_answers::SavedQuizAnswer;

#[derive(Serialize)]
pub struct AttemptQuizOption {
    pub option_id: i32,
    pub option_text: String,
    pub position: i32,
}

#[derive(Serialize)]
pub struct AttemptQuizQuestion {
    pub question_id: i32,
    pub question_type: QuestionType,
    pub question_text: String,
    pub position: i32,
    pub points: i32,
    pub options: Vec<AttemptQuizOption>,
}

#[derive(Serialize)]
pub struct AttemptAccess {
    pub can_attempt: bool,
    pub preview_only: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct AttemptTimer {
    pub time_limit_minutes: Option<i32>,
    pub expires_at: Option<String>,
    pub remaining_seconds: Option<i64>,
    pub message: String,
}

#[derive(Serialize)]
pub struct StartAttemptResponse {
    pub quiz: quiz::Model,
    pub questions: Vec<AttemptQuizQuestion>,
    pub access: AttemptAccess,
    pub timer: AttemptTimer,
    pub attempt: Option<quiz_attempts::Model>,
    pub answers: Vec<SavedQuizAnswer>,
}

#[derive(Serialize)]
pub struct QuizAttemptReviewAnswer {
    pub answer_id: Option<i32>,
    pub question_id: i32,
    pub question_type: QuestionType,
    pub question_text: String,
    pub points: i32,
    pub selected_option_id: Option<i32>,
    pub selected_option_text: Option<String>,
    pub correct_option_id: Option<i32>,
    pub correct_option_text: Option<String>,
    pub answer_text: Option<String>,
    pub score: Option<i32>,
    pub feedback: Option<String>,
}

#[derive(Serialize)]
pub struct StaffQuizAttempt {
    pub attempt_id: i32,
    pub quiz_id: i32,
    pub user_id: i32,
    pub student_name: String,
    pub student_email: String,
    pub started_at: NaiveDateTime,
    pub submitted_at: Option<NaiveDateTime>,
    pub total_score: Option<i32>,
    pub max_score: i32,
    pub is_graded: bool,
    pub answers: Vec<QuizAttemptReviewAnswer>,
}

#[derive(Serialize)]
pub struct StudentQuizAttemptReview {
    pub attempt_id: i32,
    pub quiz_id: i32,
    pub total_score: Option<i32>,
    pub max_score: i32,
    pub submitted_at: Option<NaiveDateTime>,
    pub answers: Vec<QuizAttemptReviewAnswer>,
}
