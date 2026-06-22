use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveQuizAnswer {
    pub question_id: i32,
    pub selected_option_id: Option<i32>,
    pub answer_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveQuizAnswers {
    pub answers: Vec<SaveQuizAnswer>,
}

#[derive(Debug, Serialize)]
pub struct SavedQuizAnswer {
    pub question_id: i32,
    pub selected_option_id: Option<i32>,
    pub answer_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GradeQuizAnswer {
    pub score: i32,
    pub feedback: String,
}
