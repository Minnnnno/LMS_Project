use actix_web::web;

use crate::controller::certificate_controller::{
    get_course_certificates, get_my_certificates, verify_certificate_token,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_my_certificates);
    cfg.service(get_course_certificates);
    cfg.service(verify_certificate_token);
}
