use actix_web::web;

use crate::{
    controller::quiz_questions_controller::{
        get_quiz_questions,
        get_qns_by_quiz_id
        }
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_quiz_questions);
    cfg.service(get_qns_by_quiz_id);
}