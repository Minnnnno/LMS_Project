use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quiz_answers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub answer_id: i32,
    pub attempt_id: i32,
    pub question_id: i32,
    pub selected_option_id: Option<i32>,
    pub answer_text: Option<String>,
    pub score: Option<i32>,
    pub feedback: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}