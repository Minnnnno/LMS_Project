use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AutosaveQuizAnswer {
    pub question_id: i32,
    pub selected_option_id: Option<i32>,
    pub answer_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AutosaveQuizAnswers {
    pub answers: Vec<AutosaveQuizAnswer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitMcqAnswer {
    pub attempt_id: i32,
    pub question_id: i32,
    pub selected_option_id: i32, // required, not optional
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitLongAnswer {
    pub attempt_id: i32,
    pub question_id: i32,
    pub answer_text: String, // required, not optional
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GradeQuizAnswer {
    pub score: i32,
    pub feedback: String,
}
