use actix_web::web;

use crate::{
    controller::course_controller::{
        get_courses,
        get_course_by_course_id,
        search_course, 
        update_course,
        create_course,
        delete_course
    }
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_courses);
    cfg.service(get_course_by_course_id);
    cfg.service(search_course);
    cfg.service(update_course);
    cfg.service(create_course);
    cfg.service(delete_course);
}