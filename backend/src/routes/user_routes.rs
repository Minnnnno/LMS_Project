use actix_web::web;

use crate::{
    controller::user_controller::{
        lecturer_signup,
        login,
        logout,
        profile,
        register, 
        register_submit}
};
use crate::controller::user_controller::login_submit;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(login);
    cfg.service(profile);
    cfg.service(register);
    cfg.service(register_submit);
    cfg.service(login_submit);
    cfg.service(logout);
    cfg.service(lecturer_signup);
}
