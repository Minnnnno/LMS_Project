use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "submissions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub submission_id: i32,
    pub assignment_id: i32,
    pub user_id: i32,
    pub submitted_at: NaiveDateTime,
    pub submission_text: Option<String>,
    pub file_url: Option<String>,
    pub cloudinary_public_id: Option<String>,
    pub score: Option<Decimal>,
    pub feedback: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
