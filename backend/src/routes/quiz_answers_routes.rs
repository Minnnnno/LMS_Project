use actix_web::web;

use crate::controller::quiz_answers_controller::{
    get_quiz_answers,
    get_answers_by_attempt_id,
    submit_mcq_answer,
    submit_long_answer,
    grade_quiz_answer,
    delete_quiz_answer,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_quiz_answers);
    cfg.service(get_answers_by_attempt_id);
    cfg.service(submit_mcq_answer);
    cfg.service(submit_long_answer);
    cfg.service(grade_quiz_answer);
    cfg.service(delete_quiz_answer);
}