use actix_web::web;

use crate::controller::quiz_answers_controller::{
    autosave_quiz_answers, delete_quiz_answer, get_answers_by_attempt_id, grade_quiz_answer,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_answers_by_attempt_id);
    cfg.service(autosave_quiz_answers);
    cfg.service(grade_quiz_answer);
    cfg.service(delete_quiz_answer);
}
