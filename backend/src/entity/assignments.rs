use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "assignments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub assignment_id: i32,

    pub course_id: i32,

    pub title: String,

    pub description: Option<String>,

    pub due_date: Option<NaiveDateTime>,

    pub max_score: Option<Decimal>,

    pub assignment_brief_url: Option<String>,

    pub expected_file_type: Option<String>,

    pub allow_text_submission: Option<bool>,

    pub allow_file_submission: Option<bool>,

    pub max_file_size_mb: Option<i32>,

    pub submission_instructions: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
