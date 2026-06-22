use actix_web::web;

use crate::controller::quiz_answers_controller::{grade_quiz_answer, save_quiz_answers};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(save_quiz_answers);
    cfg.service(grade_quiz_answer);
}
