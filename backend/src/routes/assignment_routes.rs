use actix_web::web;

use crate::controller::assignment_controller::get_assignment;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_assignment);
}