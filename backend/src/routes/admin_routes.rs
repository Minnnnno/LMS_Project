use actix_web::web;

use crate::controller::admin_controller::{
    admin_analytics_page, admin_approve_organisation_signup_request, admin_courses_page,
    admin_create_course, admin_create_user, admin_dashboard, admin_delete_course,
    admin_delete_user, admin_enroll_user, admin_enrollments_page, admin_get_analytics_data,
    admin_get_courses, admin_get_enrollments, admin_get_organisation_signup_requests,
    admin_get_roles, admin_get_users, admin_organisations_page,
    admin_reject_organisation_signup_request, admin_stats, admin_unenroll_user, admin_update_course,
    admin_update_user, admin_users_page, create_organisation, delete_organisation,
    get_organisations, update_organisation,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(admin_dashboard);
    cfg.service(admin_analytics_page);
    cfg.service(admin_organisations_page);
    cfg.service(admin_users_page);
    cfg.service(admin_courses_page);
    cfg.service(admin_enrollments_page);
    cfg.service(admin_get_analytics_data);
    cfg.service(admin_get_roles);
    cfg.service(admin_stats);

    cfg.service(get_organisations);
    cfg.service(admin_get_organisation_signup_requests);
    cfg.service(admin_approve_organisation_signup_request);
    cfg.service(admin_reject_organisation_signup_request);
    cfg.service(create_organisation);
    cfg.service(update_organisation);
    cfg.service(delete_organisation);

    cfg.service(admin_get_users);
    cfg.service(admin_create_user);
    cfg.service(admin_update_user);
    cfg.service(admin_delete_user);

    cfg.service(admin_get_courses);
    cfg.service(admin_create_course);
    cfg.service(admin_update_course);
    cfg.service(admin_delete_course);

    cfg.service(admin_get_enrollments);
    cfg.service(admin_enroll_user);
    cfg.service(admin_unenroll_user);
}
