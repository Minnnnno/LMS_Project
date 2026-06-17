use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Serialize)]
pub struct CourseGradebook {
    pub assignments: Vec<AssignmentGrade>,
    pub quizzes: Vec<QuizGrade>,
    pub quiz_message: Option<String>,
}

#[derive(Serialize)]
pub struct AssignmentGrade {
    pub assignment_id: i32,
    pub title: String,
    pub due_date: Option<NaiveDateTime>,
    pub max_score: Option<Decimal>,
    pub score: Option<Decimal>,
    pub feedback: Option<String>,
    pub submitted_at: Option<NaiveDateTime>,
}

#[derive(Serialize)]
pub struct QuizGrade {
    pub quiz_id: i32,
    pub title: String,
    pub max_score: i32,
    pub total_score: Option<i32>,
    pub submitted_at: Option<NaiveDateTime>,
    pub attempt_id: Option<i32>,
    pub is_graded: bool,
}
