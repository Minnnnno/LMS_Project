use actix_web::web;
use crate::controller::cloudinary_controller::upload_file;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(upload_file);
}