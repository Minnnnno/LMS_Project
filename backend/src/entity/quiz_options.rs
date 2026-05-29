use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quiz_options")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub option_id: i32,
    pub question_id: i32,
    pub option_text: String,
    pub is_correct: bool,
    pub position: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}