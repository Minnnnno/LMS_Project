use actix_web::web;

use crate::controller::user_controller::login_submit;
use crate::controller::user_controller::{
    change_password_page, change_password_submit, debug_session,
    forgot_password_page, forgot_password_submit,
    google_auth, google_callback,
    lecturer_signup, login, logout, profile, register, register_submit, resend_verification_email,
    reset_password_page, reset_password_submit,
    verify_email,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(login);
    cfg.service(change_password_page);
    cfg.service(change_password_submit);
    cfg.service(forgot_password_page);
    cfg.service(forgot_password_submit);
    cfg.service(reset_password_page);
    cfg.service(reset_password_submit);
    cfg.service(profile);
    cfg.service(register);
    cfg.service(register_submit);
    cfg.service(login_submit);
    cfg.service(google_auth);
    cfg.service(google_callback);
    cfg.service(verify_email);
    cfg.service(resend_verification_email);
    cfg.service(logout);
    cfg.service(lecturer_signup);
    cfg.service(debug_session);
}
