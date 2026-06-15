use actix_web::web;

use crate::controller::grade_controller::get_my_course_grades;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_my_course_grades);
}
