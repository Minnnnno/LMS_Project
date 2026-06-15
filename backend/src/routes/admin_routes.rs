use actix_web::web;

use crate::controller::admin_controller::{
    admin_dashboard,
    admin_get_roles,

    get_organisations,
    create_organisation,
    update_organisation,
    delete_organisation,

    admin_get_users,
    admin_get_user_by_id,
    admin_create_user,
    admin_update_user,
    admin_delete_user,

    admin_get_courses,
    admin_get_course_by_id,
    admin_create_course,
    admin_update_course,
    admin_delete_course,

    admin_get_enrollments,
    admin_enroll_user,
    admin_unenroll_user,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(admin_dashboard);
    cfg.service(admin_get_roles);

    cfg.service(get_organisations);
    cfg.service(create_organisation);
    cfg.service(update_organisation);
    cfg.service(delete_organisation);

    cfg.service(admin_get_users);
    cfg.service(admin_get_user_by_id);
    cfg.service(admin_create_user);
    cfg.service(admin_update_user);
    cfg.service(admin_delete_user);

    cfg.service(admin_get_courses);
    cfg.service(admin_get_course_by_id);
    cfg.service(admin_create_course);
    cfg.service(admin_update_course);
    cfg.service(admin_delete_course);

    cfg.service(admin_get_enrollments);
    cfg.service(admin_enroll_user);
    cfg.service(admin_unenroll_user);
}
