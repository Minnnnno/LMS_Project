use actix_web::web;

use crate::{
    controller::quiz_attempts_controller::{
        get_quiz_attempts,
        get_attempts_by_quiz_id,
        create_quiz_attempt,
        submit_quiz_attempt,
        grade_attempt,
        delete_quiz_attempt,
        }
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_quiz_attempts);
    cfg.service(get_attempts_by_quiz_id);
    cfg.service(create_quiz_attempt);
    cfg.service(submit_quiz_attempt);
    cfg.service(grade_attempt);
    cfg.service(delete_quiz_attempt);
}