use actix_web::web;

use crate::controller::organisation_controller::{
    assign_course_instructor, create_organisation, delete_organisation, get_organisation,
    invite_instructor, list_all_users, list_course_instructors, list_org_members,
    list_organisations, list_unassigned_users, mass_enroll, remove_course_instructor,
    remove_org_member,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(list_organisations)
        .service(get_organisation)
        .service(create_organisation)
        .service(delete_organisation)
        .service(list_org_members)
        .service(invite_instructor)
        .service(list_course_instructors)
        .service(assign_course_instructor)
        .service(remove_course_instructor)
        .service(mass_enroll)
        .service(remove_org_member)
        .service(list_all_users)
        .service(list_unassigned_users);
}
