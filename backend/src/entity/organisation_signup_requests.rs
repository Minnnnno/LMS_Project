use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "organisation_signup_requests")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub request_id: i32,
    pub org_name: String,
    pub org_slug: String,
    pub org_type: Option<String>,
    pub website_url: Option<String>,
    pub requester_user_id: Option<i32>,
    pub admin_first_name: Option<String>,
    pub admin_last_name: Option<String>,
    pub admin_email: String,
    pub admin_password_hash: Option<String>,
    pub status: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTimeWithTimeZone>,
    pub rejected_by: Option<i32>,
    pub rejected_at: Option<DateTimeWithTimeZone>,
    pub rejection_reason: Option<String>,
    pub created_at: Option<DateTimeWithTimeZone>,
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
