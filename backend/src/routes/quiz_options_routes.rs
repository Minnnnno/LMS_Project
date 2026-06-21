use actix_web::web;

use crate::controller::quiz_options_controller::get_options_by_qn_id;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_options_by_qn_id);
}
