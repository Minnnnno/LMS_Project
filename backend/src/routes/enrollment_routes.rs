use actix_web::web;

use crate::controller::enrollment_controller::enroll_free_course;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(enroll_free_course);
}