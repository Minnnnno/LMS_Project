use actix_web::web;

use crate::controller::quiz_controller::{
    create_quiz, create_quiz_draft, delete_quiz, get_quiz, get_quiz_attempt_view,
    get_quiz_by_course_id, update_quiz, update_quiz_draft,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_quiz);
    cfg.service(get_quiz_by_course_id);
    cfg.service(get_quiz_attempt_view);
    cfg.service(create_quiz);
    cfg.service(create_quiz_draft);
    cfg.service(update_quiz_draft);
    cfg.service(update_quiz);
    cfg.service(delete_quiz);
}
