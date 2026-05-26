use actix_web::web;

use crate::{
    controller::user_controller::{
    login,
        register, 
        register_submit}
};
use crate::controller::user_controller::login_submit;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(login);
    cfg.service(register);
    cfg.service(register_submit);
    cfg.service(login_submit);
}