use actix_web::web;

use crate::controller::organisation_controller::{
    create_organisation,
    delete_organisation,
    get_organisation,
    list_org_members,
    list_organisations,
    list_unassigned_users,
    mass_enroll,
    organisation_page,
    remove_org_member,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(organisation_page)
        .service(list_organisations)
        .service(get_organisation)
        .service(create_organisation)
        .service(delete_organisation)
        .service(list_org_members)
        .service(mass_enroll)
        .service(remove_org_member)
        .service(list_unassigned_users);
}
