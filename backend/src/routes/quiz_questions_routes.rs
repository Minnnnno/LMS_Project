use actix_web::web;

use crate::controller::quiz_questions_controller::get_qns_by_quiz_id;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_qns_by_quiz_id);
}
