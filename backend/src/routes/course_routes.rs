use actix_web::web;

use crate::controller::course_controller::{
    create_course, delete_course, get_course_by_course_id, get_course_manage_access,
    get_course_module_progress, get_course_progress, get_courses, get_my_courses,
    get_organisation_courses, search_course, update_course,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_courses);
    cfg.service(get_organisation_courses);
    cfg.service(get_my_courses);
    cfg.service(get_course_by_course_id);
    cfg.service(get_course_manage_access);
    cfg.service(get_course_module_progress);
    cfg.service(get_course_progress);
    cfg.service(search_course);
    cfg.service(update_course);
    cfg.service(create_course);
    cfg.service(delete_course);
}
