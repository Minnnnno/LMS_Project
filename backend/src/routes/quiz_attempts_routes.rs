use actix_web::web;

use crate::controller::quiz_attempts_controller::{
    create_quiz_attempt, get_attempts_by_quiz_id, get_my_attempt_review,
    get_my_attempt_statuses_by_course, submit_quiz_attempt,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_attempts_by_quiz_id);
    cfg.service(get_my_attempt_review);
    cfg.service(get_my_attempt_statuses_by_course);
    cfg.service(create_quiz_attempt);
    cfg.service(submit_quiz_attempt);
}
