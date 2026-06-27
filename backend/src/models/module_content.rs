use crate::entity::module_contents::{ContentCategoryEnum, ContentTypeEnum};
use chrono::{DateTime, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::DerivePartialModel;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct ModuleContent {
    pub module_content_id: i32,
    pub module_id: i32,
    pub content_type: ContentTypeEnum,
    pub content_category: Option<ContentCategoryEnum>,
    pub title: String,
    pub content_url: Option<String>,
    pub cloudinary_public_id: Option<String>,
    pub position: i32,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateModuleContent {
    pub module_id: Option<i32>,
    pub content_type: Option<ContentTypeEnum>,
    pub content_category: Option<ContentCategoryEnum>,
    pub title: Option<String>,
    pub content_url: Option<String>,
    pub cloudinary_public_id: Option<String>,
    pub position: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateModuleContent {
    pub module_id: i32,
    pub content_type: ContentTypeEnum,
    pub content_category: Option<ContentCategoryEnum>,
    pub title: String,
    pub content_url: Option<String>,
    pub cloudinary_public_id: Option<String>,
    pub position: i32,
}
