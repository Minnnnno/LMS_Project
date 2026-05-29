use serde::{Deserialize, Serialize};
use crate::entity::quiz_questions::{
    QuestionType
};

#[derive(Serialize, Deserialize)]
pub struct QuizQuestion {
    pub question_id: i32,
    pub quiz_id: i32,
    pub question_type: QuestionType,
    pub question_text: String,
    pub position: i32,
    pub points: i32,
}

#[derive(Serialize, Deserialize)]
pub struct CreateQuizQuestion {
    pub quiz_id: i32,
    pub question_type: QuestionType,
    pub question_text: String,
    pub position: i32,
    pub points: Option<i32>,  // in handler: points.unwrap_or(1)
}

#[derive(Serialize, Deserialize)]
pub struct UpdateQuizQuestion {
    pub question_type: Option<QuestionType>,
    pub question_text: Option<String>,
    pub position: Option<i32>,
    pub points: Option<i32>,
}