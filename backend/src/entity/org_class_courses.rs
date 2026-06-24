use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "org_class_courses")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub class_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub course_id: i32,
    pub assigned_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
