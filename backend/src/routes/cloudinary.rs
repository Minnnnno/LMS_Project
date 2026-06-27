use crate::controller::cloudinary_controller::upload_file;
use actix_web::web;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(upload_file);
}
