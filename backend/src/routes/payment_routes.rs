use actix_web::web;

use crate::controller::payment_controller::{
    create_checkout_session,
    payment_success,
    payment_cancelled,
    stripe_webhook,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(create_checkout_session);
    cfg.service(payment_success);
    cfg.service(payment_cancelled);
    cfg.service(stripe_webhook);
}