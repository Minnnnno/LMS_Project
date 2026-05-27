use actix_web::web;

use crate::controller::student_controller::{
    get_own_profile,
    update_own_profile,
    change_password,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_own_profile);
    cfg.service(update_own_profile);
    cfg.service(change_password);
}