use actix_session::Session;
use actix_web::HttpResponse;

use crate::services::course_service::has_role;

pub fn require_org_admin(session: &Session) -> Result<(), HttpResponse> {
    if has_role(session, "Organisation Admin") || has_role(session, "LMS Admin") {
        Ok(())
    } else {
        Err(HttpResponse::Forbidden().body("Organisation Admin or LMS Admin role required"))
    }
}
