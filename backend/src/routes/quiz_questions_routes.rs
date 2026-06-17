use actix_web::web;

use crate::{
    controller::quiz_questions_controller::{
        get_qns_by_quiz_id,
        create_quiz_qn,
        update_quiz_qn,
        delete_quiz_qn,
        }
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_qns_by_quiz_id);
    cfg.service(create_quiz_qn);
    cfg.service(update_quiz_qn);
    cfg.service(delete_quiz_qn);
}
