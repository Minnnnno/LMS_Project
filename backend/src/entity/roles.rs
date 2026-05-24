use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "roles")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub role_id: i32,

    pub role_name: RoleName,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "role_name"
)]
pub enum RoleName {

    #[sea_orm(string_value = "LMS Admin")]
    LmsAdmin,

    #[sea_orm(string_value = "Organisation Admin")]
    OrganisationAdmin,

    #[sea_orm(string_value = "Instructor")]
    Instructor,

    #[sea_orm(string_value = "Student")]
    Student,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No relations defined")
    }
}

impl ActiveModelBehavior for ActiveModel {}