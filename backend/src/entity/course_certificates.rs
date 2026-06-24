use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "course_certificates")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub certificate_id: i32,
    pub user_id: i32,
    pub course_id: i32,
    pub verification_token: Uuid,
    pub issued_at: DateTime<Utc>,
    pub completion_source: String,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
