use sea_orm::DerivePartialModel;
use chrono::{DateTime, Utc, NaiveDateTime};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct CreateCourse {
    pub name: String,
    pub instructor_id: i32,
    pub org_id: i32,
    pub status: String,
    pub price_cents: i32,
    pub currency: String,
    pub is_paid: bool,
    pub description: Option<String>,
    pub background_image_url: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct CourseQuery {
    pub name: Option<String>,
    pub instructor_id: Option<i32>,
    pub min_price: Option<i32>,
    pub max_price: Option<i32>,
    pub course_id: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateCourse{ 
    pub name: Option<String>, 
    pub instructor_id: Option<i32>,
    pub org_id: Option<i32>, 
    pub status: Option<String>, 
    pub price_cents: Option<i32>, 
    pub currency: Option<String>, 
    pub is_paid: Option<bool>,
    pub description: Option<String>,
    pub background_image_url: Option<String>,
}
