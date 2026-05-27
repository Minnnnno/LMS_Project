use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "module_contents")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub module_content_id: i32,
    pub module_id: i32,
    pub content_type: ContentTypeEnum,
    pub content_category: Option<ContentCategoryEnum>,
    pub title: String,
    pub content_url: Option<String>,
    pub cloudinary_public_id: Option<String>,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "content_type_enum")]
pub enum ContentTypeEnum {
    #[sea_orm(string_value = "video")]
    Video,

    #[sea_orm(string_value = "pdf")]
    Pdf,

    #[sea_orm(string_value = "image")]
    Image,

    #[sea_orm(string_value = "document")]
    Document,

    #[sea_orm(string_value = "link")]
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "content_category_enum")]
pub enum ContentCategoryEnum {
    #[sea_orm(string_value = "lecture")]
    Lecture,

    #[sea_orm(string_value = "tutorial")]
    Tutorial,

    #[sea_orm(string_value = "assignment")]
    Assignment,

    #[sea_orm(string_value = "reading")]
    Reading,

    #[sea_orm(string_value = "quiz")]
    Quiz,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}