use actix_web::web;

use crate::controller::enrollment_controller::{enroll_free_course, get_enrollment_status};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(enroll_free_course);
    cfg.service(get_enrollment_status);
}
