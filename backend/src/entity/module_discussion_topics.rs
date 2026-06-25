use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "module_discussion_topics")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub topic_id: i32,
    pub module_id: i32,
    pub created_by: i32,
    pub title: String,
    pub description: Option<String>,
    pub is_locked: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
