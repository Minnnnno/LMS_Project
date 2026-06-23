use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "enrollments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i32,

    #[sea_orm(primary_key, auto_increment = false)]
    pub course_id: i32,

    pub enrolled_at: DateTime<Utc>,

    pub stripe_checkout_session_id: Option<String>,

    pub paid_at: Option<DateTime<Utc>>,

    pub manual_completed_at: Option<DateTime<Utc>>,

    pub manual_completed_by: Option<i32>,

    pub manual_completion_note: Option<String>,

    pub created_at: Option<DateTimeWithTimeZone>,

    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
