use actix_web::web;

use crate::controller::certificate_controller::{
    download_my_certificate, get_course_certificates, get_my_certificates, verify_certificate_token,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_my_certificates);
    cfg.service(download_my_certificate);
    cfg.service(get_course_certificates);
    cfg.service(verify_certificate_token);
}
