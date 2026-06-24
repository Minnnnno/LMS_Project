use actix_web::web;

use crate::controller::organisation_controller::{
    add_org_class_members, assign_course_instructor, create_org_class, create_organisation,
    delete_org_class, delete_organisation, get_organisation, import_org_class_members,
    invite_instructor, list_all_users, list_course_instructors, list_org_classes,
    list_org_members, list_organisations, list_unassigned_users, mass_enroll,
    remove_course_instructor, remove_org_class_member, remove_org_member, update_org_class,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(list_organisations)
        .service(get_organisation)
        .service(create_organisation)
        .service(delete_organisation)
        .service(list_org_members)
        .service(invite_instructor)
        .service(list_org_classes)
        .service(create_org_class)
        .service(update_org_class)
        .service(delete_org_class)
        .service(add_org_class_members)
        .service(remove_org_class_member)
        .service(import_org_class_members)
        .service(list_course_instructors)
        .service(assign_course_instructor)
        .service(remove_course_instructor)
        .service(mass_enroll)
        .service(remove_org_member)
        .service(list_all_users)
        .service(list_unassigned_users);
}
