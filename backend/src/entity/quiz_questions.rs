use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quiz_questions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub question_id: i32,
    pub quiz_id: i32,
    pub question_type: QuestionType,
    pub question_text: String,
    pub position: i32,
    pub points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "question_type")]
pub enum QuestionType {
    #[serde(rename = "mcq")]
    #[sea_orm(string_value = "mcq")]
    Mcq,

    #[serde(rename = "long_answer")]
    #[sea_orm(string_value = "long_answer")]
    LongAnswer,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
