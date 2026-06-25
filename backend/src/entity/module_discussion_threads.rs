use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "module_discussion_threads")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub thread_id: i32,
    pub topic_id: i32,
    pub author_id: i32,
    pub title: String,
    pub body: String,
    pub status: String,
    pub view_count: i32,
    pub closed_by: Option<i32>,
    pub closed_at: Option<DateTimeWithTimeZone>,
    pub hidden_by: Option<i32>,
    pub hidden_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
