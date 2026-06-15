use actix_session::Session;
use actix_web::{delete, get, post, put, web, Responder};
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait};
use serde::Serialize;

use crate::app_state::AppState;
use crate::entity::{courses, enrollments, organisations, users};
use crate::models::admin::{
    CreateOrganisationForm,
    UpdateOrganisationForm,
    CreateAdminUserForm,
    UpdateAdminUserForm,
    CreateAdminCourseForm,
    UpdateAdminCourseForm,
    AdminEnrollmentForm,
};

use crate::services::auth_helpers::require_admin;
use crate::ssr::pages::render_page;

use crate::services::admin_service::{
    get_all_organisations,
    get_all_roles,
    create_organisation_service,
    update_organisation_service,
    delete_organisation_service,

    get_all_users,
    get_user_by_id_service,
    create_user_service,
    update_user_service,
    delete_user_service,

    get_all_courses,
    get_course_by_id_service,
    create_course_service,
    update_course_service,
    delete_course_service,

    get_all_enrollments,
    admin_enroll_user_service,
    admin_unenroll_user_service,
};

#[derive(Serialize)]
struct AdminStats {
    active_viewers: usize,
    total_accounts: u64,
    total_users: u64,
    total_organisations: u64,
    total_courses: u64,
    total_enrollments: u64,
}

#[get("/admin/stats")]
pub async fn admin_stats(
    db: web::Data<DatabaseConnection>,
    state: web::Data<AppState>,
    session: Session,
) -> impl Responder {
    if let Err(response) = require_admin(&session) {
        return response;
    }

    let (users, organisations, courses, enrollments) = tokio::join!(
        users::Entity::find().count(db.get_ref()),
        organisations::Entity::find().count(db.get_ref()),
        courses::Entity::find().count(db.get_ref()),
        enrollments::Entity::find().count(db.get_ref()),
    );

    match (users, organisations, courses, enrollments) {
        (Ok(total_users), Ok(total_organisations), Ok(total_courses), Ok(total_enrollments)) => {
            actix_web::HttpResponse::Ok().json(AdminStats {
                active_viewers: state.active_viewers(),
                total_accounts: total_users,
                total_users,
                total_organisations,
                total_courses,
                total_enrollments,
            })
        }
        _ => actix_web::HttpResponse::InternalServerError().body("Unable to load admin statistics"),
    }
}

#[get("/admin/roles")]
pub async fn admin_get_roles(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => get_all_roles(db.get_ref()).await,
        Err(response) => response,
    }
}

#[get("/admin/dashboard")]
pub async fn admin_dashboard(
    session: Session,
) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => render_page("admin_dashboard.html", &session),
        Err(response) => response,
    }
}

#[get("/admin/manage/organisations")]
pub async fn admin_organisations_page(session: Session) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => render_page("admin_dashboard.html", &session),
        Err(response) => response,
    }
}

#[get("/admin/manage/users")]
pub async fn admin_users_page(session: Session) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => render_page("admin_dashboard.html", &session),
        Err(response) => response,
    }
}

#[get("/admin/manage/courses")]
pub async fn admin_courses_page(session: Session) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => render_page("admin_dashboard.html", &session),
        Err(response) => response,
    }
}

#[get("/admin/manage/enrollments")]
pub async fn admin_enrollments_page(session: Session) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => render_page("admin_dashboard.html", &session),
        Err(response) => response,
    }
}

// Organisation Routes
#[get("/admin/organisations")]
pub async fn get_organisations(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => get_all_organisations(db.get_ref()).await,
        Err(response) => response,
    }
}

#[post("/admin/organisations")]
pub async fn create_organisation(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateOrganisationForm>,
) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => create_organisation_service(db.get_ref(), body.into_inner()).await,
        Err(response) => response,
    }
}

#[put("/admin/organisations/{org_id}")]
pub async fn update_organisation(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateOrganisationForm>,
) -> impl Responder {
    let org_id = path.into_inner();

    match require_admin(&session) {
        Ok(_) => update_organisation_service(db.get_ref(), org_id, body.into_inner()).await,
        Err(response) => response,
    }
}

#[delete("/admin/organisations/{org_id}")]
pub async fn delete_organisation(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let org_id = path.into_inner();

    match require_admin(&session) {
        Ok(_) => delete_organisation_service(db.get_ref(), org_id).await,
        Err(response) => response,
    }
}

// User Routes
#[get("/admin/users")]
pub async fn admin_get_users(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => get_all_users(db.get_ref()).await,
        Err(response) => response,
    }
}

#[get("/admin/users/{user_id}")]
pub async fn admin_get_user_by_id(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let user_id = path.into_inner();

    match require_admin(&session) {
        Ok(_) => get_user_by_id_service(db.get_ref(), user_id).await,
        Err(response) => response,
    }
}

#[post("/admin/users")]
pub async fn admin_create_user(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateAdminUserForm>,
) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => create_user_service(db.get_ref(), body.into_inner()).await,
        Err(response) => response,
    }
}

#[put("/admin/users/{user_id}")]
pub async fn admin_update_user(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateAdminUserForm>,
) -> impl Responder {
    let user_id = path.into_inner();

    match require_admin(&session) {
        Ok(_) => update_user_service(db.get_ref(), user_id, body.into_inner()).await,
        Err(response) => response,
    }
}

#[delete("/admin/users/{user_id}")]
pub async fn admin_delete_user(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let user_id = path.into_inner();

    match require_admin(&session) {
        Ok(_) => delete_user_service(db.get_ref(), user_id).await,
        Err(response) => response,
    }
}

// Course Routes
#[get("/admin/courses")]
pub async fn admin_get_courses(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => get_all_courses(db.get_ref()).await,
        Err(response) => response,
    }
}

#[get("/admin/courses/{course_id}")]
pub async fn admin_get_course_by_id(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let course_id = path.into_inner();

    match require_admin(&session) {
        Ok(_) => get_course_by_id_service(db.get_ref(), course_id).await,
        Err(response) => response,
    }
}

#[post("/admin/courses")]
pub async fn admin_create_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateAdminCourseForm>,
) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => create_course_service(db.get_ref(), body.into_inner()).await,
        Err(response) => response,
    }
}

#[put("/admin/courses/{course_id}")]
pub async fn admin_update_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateAdminCourseForm>,
) -> impl Responder {
    let course_id = path.into_inner();

    match require_admin(&session) {
        Ok(_) => update_course_service(db.get_ref(), course_id, body.into_inner()).await,
        Err(response) => response,
    }
}

#[delete("/admin/courses/{course_id}")]
pub async fn admin_delete_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let course_id = path.into_inner();

    match require_admin(&session) {
        Ok(_) => delete_course_service(db.get_ref(), course_id).await,
        Err(response) => response,
    }
}

// Enrollment Routes
#[get("/admin/enrollments")]
pub async fn admin_get_enrollments(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => get_all_enrollments(db.get_ref()).await,
        Err(response) => response,
    }
}

#[post("/admin/enrollments")]
pub async fn admin_enroll_user(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<AdminEnrollmentForm>,
) -> impl Responder {
    match require_admin(&session) {
        Ok(_) => admin_enroll_user_service(db.get_ref(), body.into_inner()).await,
        Err(response) => response,
    }
}

#[delete("/admin/enrollments/{user_id}/{course_id}")]
pub async fn admin_unenroll_user(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32)>,
) -> impl Responder {
    let (user_id, course_id) = path.into_inner();

    match require_admin(&session) {
        Ok(_) => admin_unenroll_user_service(db.get_ref(), user_id, course_id).await,
        Err(response) => response,
    }
}
