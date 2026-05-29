use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
pub struct QuizOptions {
    pub option_id: i32,
    pub question_id: i32,
    pub option_text: String,
    pub is_correct: bool,
    pub position: i32,
}

#[derive(Serialize, Deserialize)]
pub struct CreateQuizOption {
    pub question_id: i32,
    pub option_text: String,
    pub is_correct: bool,
    pub position: i32,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateQuizOption {
    pub option_text: Option<String>,
    pub is_correct: Option<bool>,
    pub position: Option<i32>,
}