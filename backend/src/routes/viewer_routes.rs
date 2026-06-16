use actix_web::web;

use crate::controller::viewer_controller::viewer_heartbeat;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(viewer_heartbeat);
}
