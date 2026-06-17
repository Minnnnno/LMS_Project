use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quiz_attempts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub attempt_id: i32,
    pub quiz_id: i32,
    pub user_id: i32,
    pub started_at: NaiveDateTime,
    pub submitted_at: Option<NaiveDateTime>,
    pub total_score: Option<i32>,
    pub is_graded: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
