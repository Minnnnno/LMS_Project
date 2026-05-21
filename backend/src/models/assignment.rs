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