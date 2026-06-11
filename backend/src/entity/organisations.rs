use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "organisations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub org_id: i32,

    pub org_name: String,

    pub org_slug: Option<String>,

    pub org_type: Option<String>,

    pub website_url: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
