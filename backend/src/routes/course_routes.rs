use actix_web::web;

use crate::{
    controller::course_controller::{
        get_courses,
        search_course, 
        update_course,
        create_course,
        delete_course
    }
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_courses);
    cfg.service(search_course);
    cfg.service(update_course);
    cfg.service(create_course);
    cfg.service(delete_course);
}