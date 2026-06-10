use actix_web::web;

use crate::{
    controller::course_controller::{
        get_courses,
        get_organisation_courses,
        get_my_courses,
        get_course_by_course_id,
        get_course_manage_access,
        search_course, 
        update_course,
        create_course,
        delete_course
    }
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_courses);
    cfg.service(get_organisation_courses);
    cfg.service(get_my_courses);
    cfg.service(get_course_by_course_id);
    cfg.service(get_course_manage_access);
    cfg.service(search_course);
    cfg.service(update_course);
    cfg.service(create_course);
    cfg.service(delete_course);
}
