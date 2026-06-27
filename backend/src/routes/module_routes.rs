use actix_web::web;

use crate::controller::module_controller::{
    create_module, delete_module, get_modules, get_modules_by_course_id, update_module,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_modules);
    cfg.service(get_modules_by_course_id);
    cfg.service(update_module);
    cfg.service(create_module);
    cfg.service(delete_module);
}
