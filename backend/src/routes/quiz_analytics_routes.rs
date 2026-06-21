use actix_web::web;

use crate::controller::quiz_analytics_controller::{get_course_quiz_analytics, get_quiz_analytics};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_course_quiz_analytics);
    cfg.service(get_quiz_analytics);
}
