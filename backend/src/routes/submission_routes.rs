use actix_web::web;

use crate::controller::submission_controller::{create_submission, list_my_submissions};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(create_submission);
    cfg.service(list_my_submissions);
}
