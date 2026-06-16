use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "courses")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub course_id: i32,
    pub instructor_id: Option<i32>,
    pub name: Option<String>,
    pub org_id: Option<i32>,
    pub status: CourseStatus,
    pub price_cents: Option<i32>,
    pub currency: Option<String>,
    pub is_paid: Option<bool>,
    pub description: Option<String>,
    pub background_image_url: Option<String>,
    pub visibility: String,
    pub created_at: Option<DateTimeWithTimeZone>,
    pub updated_at: Option<DateTimeWithTimeZone>,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "course_status"
)]
pub enum CourseStatus {
    #[sea_orm(string_value = "draft")]
    Draft,

    #[sea_orm(string_value = "published")]
    Published,

    #[sea_orm(string_value = "archived")]
    Archived,
}
#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No relations defined")
    }
}

impl ActiveModelBehavior for ActiveModel {}
