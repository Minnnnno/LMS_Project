use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "lesson_contents")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub content_id: i32,
    pub lesson_id: i32,
    pub content_type: String,
    pub title: String,
    pub content_url: String,
    pub cloudinary_public_id: String,
    pub content_body: String,
    pub position: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}