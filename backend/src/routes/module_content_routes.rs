use actix_web::web;

use crate::{
    controller::module_content_controller::{
        get_module_contents,
        get_module_content_by_id,
        get_module_content_manage_access,
        get_module_content_progress,
        mark_module_content_opened,
        update_module_content,
        create_module_content,
        delete_module_content
    }
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_module_contents);
    cfg.service(get_module_content_manage_access);
    cfg.service(get_module_content_progress);
    cfg.service(mark_module_content_opened);
    cfg.service(get_module_content_by_id);
    cfg.service(update_module_content);
    cfg.service(create_module_content);
    cfg.service(delete_module_content);
}
