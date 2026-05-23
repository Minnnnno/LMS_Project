use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub payment_id: i32,

    pub user_id: i32,
    pub course_id: i32,

    pub provider: String,

    pub checkout_session_id: Option<String>,
    pub payment_ref: Option<String>,

    pub amount_cents: i32,
    pub currency: String,

    pub payment_status: String,

    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}