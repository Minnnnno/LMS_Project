use actix_web::web;

use crate::{
    controller::quiz_questions_controller::{
        get_quiz_questions,
        }
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_quiz_questions);
}