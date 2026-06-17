use actix_web::web;

use crate::{
    controller::quiz_options_controller::{
        get_options_by_qn_id,
        create_quiz_option,
        update_quiz_option,
        delete_quiz_option,
        }
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_options_by_qn_id);
    cfg.service(create_quiz_option);
    cfg.service(update_quiz_option);
    cfg.service(delete_quiz_option);
}
