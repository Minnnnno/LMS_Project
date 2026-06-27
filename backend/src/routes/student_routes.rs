use actix_web::web;

use crate::controller::student_controller::{change_password, get_own_profile, update_own_profile};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_own_profile);
    cfg.service(update_own_profile);
    cfg.service(change_password);
}
