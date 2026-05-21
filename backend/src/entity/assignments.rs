use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDateTime};
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "assignments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub assignment_id: i32,
    pub course_id: i32,
    pub title: String,
    pub description: String,
    pub due_date: NaiveDateTime,
    pub max_score: Decimal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}