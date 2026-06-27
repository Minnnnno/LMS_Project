use actix_web::web;

use crate::controller::assignment_controller::{
    create_assignment, delete_assignment, get_assignment, get_assignment_by_course_id,
    update_assignment,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_assignment_by_course_id);
    cfg.service(get_assignment);
    cfg.service(update_assignment);
    cfg.service(create_assignment);
    cfg.service(delete_assignment);
}
