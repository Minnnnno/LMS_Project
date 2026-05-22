use sea_orm::DerivePartialModel;
use chrono::{DateTime, Utc, NaiveDateTime};
use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;

#[derive(Serialize, Deserialize)]
pub struct assignment {
    pub assignment_id: i32, 
    pub course_id: i32, 
    pub title: String, 
    pub description: String, 
    pub due_date: NaiveDateTime, 
    pub max_score: Decimal
}

#[derive(Serialize, Deserialize)]
pub struct UpdateAssignment{
    pub course_id: Option<i32>, 
    pub title: Option<String>, 
    pub description: Option<String>, 
    pub due_date: Option<NaiveDateTime>, 
    pub max_score: Option<Decimal> 
}

#[derive(Serialize, Deserialize)]
pub struct CreateAssignment {
    pub course_id: i32, 
    pub title: String, 
    pub description: String, 
    pub due_date: NaiveDateTime, 
    pub max_score: Decimal
}