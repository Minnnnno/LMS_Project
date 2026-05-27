use sea_orm::DerivePartialModel;
use chrono::{DateTime, Utc, NaiveDateTime};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct CreateModules {
    pub course_id: i32, 
    pub title: String, 
    pub position: i32
}
#[derive(Serialize, Deserialize)]
pub struct ModulesQuery {
    pub module_id: Option<i32>,
    pub course_id: Option<i32>,
    pub title: Option<String>,
    pub position: Option<i32>
}

#[derive(Serialize, Deserialize)]
pub struct UpdateModules{ 
    pub module_id: Option<i32>,
    pub course_id: Option<i32>,
    pub title: Option<String>,
    pub position: Option<i32>
}
