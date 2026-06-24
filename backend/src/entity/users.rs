use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub auth_provider: String,
    pub org_id: Option<i32>,
    pub email_verified: bool,
    pub must_change_password: bool,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTimeWithTimeZone>,
    pub created_at: Option<DateTimeWithTimeZone>,
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
