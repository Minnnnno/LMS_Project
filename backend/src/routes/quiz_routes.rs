use actix_web::web;

use crate::{
    controller::quiz_controller::{
        get_quiz,
        get_quiz_by_course_id,
        create_quiz,
        update_quiz,
        delete_quiz,
        }
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_quiz);
    cfg.service(get_quiz_by_course_id);
    cfg.service(create_quiz);
    cfg.service(update_quiz);
    cfg.service(delete_quiz);
}