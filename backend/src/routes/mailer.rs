use actix_web::web; 
use crate::controller::mailer_controller::send_mail; 

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.route("/mailer/send", web::post().to(send_mail));
}