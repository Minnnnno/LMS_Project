use actix_web::web;

use crate::controller::quiz_controller::{
    create_quiz_draft, delete_quiz, get_quiz_by_course_id, get_quiz_draft, update_quiz_draft,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_quiz_by_course_id);
    cfg.service(create_quiz_draft);
    cfg.service(get_quiz_draft);
    cfg.service(update_quiz_draft);
    cfg.service(delete_quiz);
}
