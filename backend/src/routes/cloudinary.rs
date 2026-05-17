use actix_web::web;
use crate::controller::cloudinary_controller::get_upload_signature;
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.route("/cloudinary/signature", web::get().to(get_upload_signature));
}