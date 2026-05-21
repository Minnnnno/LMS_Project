use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "submissions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub submission_id: i32,
    pub assignment_id: i32,
    pub user_id: i32,
    pub submitted_at: DateTime<Utc>,
    pub submission_text: String,
    pub file_url: String,
    pub cloudinary_public_id: String,
    pub score: Decimal,
    pub feedback: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}