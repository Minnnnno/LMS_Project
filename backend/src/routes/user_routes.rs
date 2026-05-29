use actix_web::web;

use crate::controller::user_controller::login_submit;
use crate::controller::user_controller::{
    google_auth, google_callback, lecturer_signup, login, logout, profile, register,
    register_submit,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(login);
    cfg.service(profile);
    cfg.service(register);
    cfg.service(register_submit);
    cfg.service(login_submit);
    cfg.service(google_auth);
    cfg.service(google_callback);
    cfg.service(logout);
    cfg.service(lecturer_signup);
}
