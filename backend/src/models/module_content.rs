use sea_orm::DerivePartialModel;
use chrono::{DateTime, Utc, NaiveDateTime};
use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;
use crate::entity::module_contents::{
    ContentTypeEnum,
    ContentCategoryEnum,
};
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
pub struct UpdateModuleContent{
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