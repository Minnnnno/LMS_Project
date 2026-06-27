use actix_session::Session;
use actix_web::{
    HttpRequest, HttpResponse, Responder, delete, get,
    http::{StatusCode, header},
    post, put, web,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use std::collections::{HashMap, HashSet};
use tera::{Context, Tera};
use validator::{Validate, validate_email};

use crate::entity::{
    course_instructors, courses, enrollments, org_class_courses, org_class_members, org_classes,
    organisation_signup_requests, organisations, roles, user_roles, users,
};
use crate::models::organisation::{
    AddClassMembersForm, AssignCourseInstructorForm, CourseInstructorCourseDto,
    CourseInstructorDto, CourseInstructorSummaryDto, CreateOrgClassForm, CreateOrganisationForm,
    ImportClassMembersForm, InviteInstructorForm, MassEnrollForm, OrgClassCourseDto, OrgClassDto,
    OrgClassMemberDto, OrgClassSummaryDto, OrgMemberDto, OrganisationSignupForm,
    UpdateOrgClassForm,
};
use crate::services::auth_helpers::redirect_to_login;
use crate::services::captcha_service::{recaptcha_site_key, verify_recaptcha};
use crate::services::course_service::{get_session_user_org_id, has_role};
use crate::services::mailer_service::{MailRequest, send_mail_message};
use crate::services::organisation_service;
use crate::services::user_service::hash_password;
use crate::ssr::pages::{build_page_context, render_page};
use uuid::Uuid;

const ORG_DASHBOARD_PATH: &str = "/organisation";

// ── Session helpers ────────────────────────────────────────────────────────────

// ── Page route ─────────────────────────────────────────────────────────────────

#[get("/organisation")]
pub async fn organisation_page(session: Session) -> impl Responder {
    match session.get::<i32>("user_id") {
        Ok(Some(_)) => {}
        Ok(None) => return redirect_to_login(),
        Err(err) => {
            println!("Session user lookup error: {:?}", err);
            return HttpResponse::InternalServerError().body("Unable to read session.");
        }
    }

    if !has_role(&session, "Organisation Admin") {
        return HttpResponse::Forbidden().body("Organisation Admin role required");
    }

    render_page("organisation.html", &session)
}

#[get("/organisations/signup")]
pub async fn organisation_signup_page(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    match current_session_user(db.get_ref(), &session).await {
        Ok(Some(user)) => {
            if let Some(org_id) = user.org_id {
                let org_name = match organisations::Entity::find_by_id(org_id)
                    .one(db.get_ref())
                    .await
                {
                    Ok(Some(org)) => Some(org.org_name),
                    Ok(None) => None,
                    Err(err) => {
                        println!("Organisation lookup error: {:?}", err);
                        None
                    }
                };

                return render_signup_page(
                    &session,
                    StatusCode::OK,
                    None,
                    Some(&user),
                    org_name.as_deref(),
                    None,
                );
            }

            if !user.email_verified {
                return render_signup_page(
                    &session,
                    StatusCode::OK,
                    Some("Please verify your email before creating an organisation."),
                    Some(&user),
                    None,
                    None,
                );
            }

            render_signup_page(&session, StatusCode::OK, None, Some(&user), None, None)
        }
        Ok(None) => render_signup_page(&session, StatusCode::OK, None, None, None, None),
        Err(response) => response,
    }
}

#[post("/organisations/signup")]
pub async fn organisation_signup_submit(
    db: web::Data<DatabaseConnection>,
    req: HttpRequest,
    session: Session,
    form: web::Form<OrganisationSignupForm>,
) -> impl Responder {
    let form = form.into_inner();
    let current_user = match current_session_user(db.get_ref(), &session).await {
        Ok(user) => user,
        Err(response) => return response,
    };
    let creates_new_admin_account = current_user.is_none();

    if let Some(user) = &current_user {
        if !user.email_verified {
            return render_signup_page(
                &session,
                StatusCode::BAD_REQUEST,
                Some("Please verify your email before creating an organisation."),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }

        if let Some(org_id) = user.org_id {
            let org_name = match organisations::Entity::find_by_id(org_id)
                .one(db.get_ref())
                .await
            {
                Ok(Some(org)) => Some(org.org_name),
                Ok(None) => None,
                Err(err) => {
                    println!("Organisation lookup error: {:?}", err);
                    None
                }
            };

            return render_signup_page(
                &session,
                StatusCode::BAD_REQUEST,
                Some("You already belong to an organisation."),
                current_user.as_ref(),
                org_name.as_deref(),
                Some(&form),
            );
        }
    }

    if let Some(message) = validate_signup_form(&form, current_user.is_none()) {
        return render_signup_page(
            &session,
            StatusCode::BAD_REQUEST,
            Some(message),
            current_user.as_ref(),
            None,
            Some(&form),
        );
    }

    match verify_recaptcha(form.recaptcha_response.as_deref(), request_ip_address(&req)).await {
        Ok(true) => {}
        Ok(false) => {
            return render_signup_page(
                &session,
                StatusCode::BAD_REQUEST,
                Some("Please complete the reCAPTCHA challenge."),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }
        Err(message) => {
            return render_signup_page(
                &session,
                StatusCode::BAD_REQUEST,
                Some(&message),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }
    }

    let org_name = form.org_name.trim().to_string();
    let org_slug = form.org_slug.trim().to_lowercase();
    let org_type = optional_trimmed(form.org_type.as_deref());
    let website_url = normalize_website_url(form.website_url.as_deref());

    match organisations::Entity::find()
        .filter(organisations::Column::OrgSlug.eq(org_slug.clone()))
        .one(db.get_ref())
        .await
    {
        Ok(Some(_)) => {
            return render_signup_page(
                &session,
                StatusCode::BAD_REQUEST,
                Some("Organisation slug already exists."),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }
        Ok(None) => {}
        Err(err) => {
            println!("Organisation slug lookup error: {:?}", err);
            return render_signup_page(
                &session,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some("Unable to check the organisation slug right now."),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }
    }

    match organisation_signup_requests::Entity::find()
        .filter(organisation_signup_requests::Column::OrgSlug.eq(org_slug.clone()))
        .filter(organisation_signup_requests::Column::Status.eq("pending"))
        .one(db.get_ref())
        .await
    {
        Ok(Some(_)) => {
            return render_signup_page(
                &session,
                StatusCode::BAD_REQUEST,
                Some("An organisation signup request with this slug is already pending approval."),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }
        Ok(None) => {}
        Err(err) => {
            println!("Pending organisation slug lookup error: {:?}", err);
            return render_signup_page(
                &session,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some("Unable to check pending organisation requests right now."),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }
    };

    let (requester_user_id, admin_first_name, admin_last_name, admin_email, admin_password_hash) =
        if let Some(user) = &current_user {
            (
                Some(user.user_id),
                Some(user.first_name.trim().to_string()),
                Some(user.last_name.trim().to_string()),
                user.email.trim().to_lowercase(),
                None,
            )
        } else {
            let admin_email = form
                .admin_email
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_lowercase();

            match users::Entity::find()
                .filter(users::Column::Email.eq(admin_email.clone()))
                .one(db.get_ref())
                .await
            {
                Ok(Some(_)) => {
                    return render_signup_page(
                        &session,
                        StatusCode::BAD_REQUEST,
                        Some("Email already exists."),
                        None,
                        None,
                        Some(&form),
                    );
                }
                Ok(None) => {}
                Err(err) => {
                    println!("Admin email lookup error: {:?}", err);
                    return render_signup_page(
                        &session,
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Some("Unable to check the admin email right now."),
                        None,
                        None,
                        Some(&form),
                    );
                }
            }

            let password_hash = match hash_password(
                form.admin_password
                    .as_deref()
                    .unwrap_or_default()
                    .to_string(),
            )
            .await
            {
                Ok(hash) => hash,
                Err(message) => {
                    return render_signup_page(
                        &session,
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Some(&message),
                        None,
                        None,
                        Some(&form),
                    );
                }
            };

            (
                None,
                Some(
                    form.admin_first_name
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                ),
                Some(
                    form.admin_last_name
                        .as_deref()
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                ),
                admin_email,
                Some(password_hash),
            )
        };

    match organisation_signup_requests::Entity::find()
        .filter(organisation_signup_requests::Column::AdminEmail.eq(admin_email.clone()))
        .filter(organisation_signup_requests::Column::Status.eq("pending"))
        .one(db.get_ref())
        .await
    {
        Ok(Some(_)) => {
            return render_signup_page(
                &session,
                StatusCode::BAD_REQUEST,
                Some(
                    "An organisation signup request for this admin email is already pending approval.",
                ),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }
        Ok(None) => {}
        Err(err) => {
            println!("Pending admin email lookup error: {:?}", err);
            return render_signup_page(
                &session,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some("Unable to check pending organisation requests right now."),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }
    }

    let signup_request = organisation_signup_requests::ActiveModel {
        org_name: Set(org_name),
        org_slug: Set(org_slug),
        org_type: Set(org_type),
        website_url: Set(website_url),
        requester_user_id: Set(requester_user_id),
        admin_first_name: Set(admin_first_name),
        admin_last_name: Set(admin_last_name),
        admin_email: Set(admin_email),
        admin_password_hash: Set(admin_password_hash),
        status: Set("pending".to_string()),
        ..Default::default()
    };

    if let Err(err) = signup_request.insert(db.get_ref()).await {
        println!("Organisation signup request insert error: {:?}", err);
        return render_signup_page(
            &session,
            StatusCode::INTERNAL_SERVER_ERROR,
            Some("Unable to submit the organisation request right now."),
            current_user.as_ref(),
            None,
            Some(&form),
        );
    }

    let success_message = if creates_new_admin_account {
        "Organisation request submitted. The LMS administrator will review it and email the organisation admin after approval."
    } else {
        "Organisation request submitted. The LMS administrator will review it and email you after approval."
    };
    let _ = session.insert("organisation_signup_success", success_message);

    HttpResponse::Found()
        .insert_header((header::LOCATION, "/organisations/signup"))
        .finish()
}

// ── CRUD: organisations ────────────────────────────────────────────────────────

async fn current_session_user(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<Option<users::Model>, HttpResponse> {
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return Ok(None),
        Err(err) => {
            println!("Session user lookup error: {:?}", err);
            return Err(HttpResponse::InternalServerError().body("Unable to read session."));
        }
    };

    users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|err| {
            println!("Current user lookup error: {:?}", err);
            HttpResponse::InternalServerError().body("Unable to load your account.")
        })
}

fn request_ip_address(req: &HttpRequest) -> Option<String> {
    req.connection_info()
        .realip_remote_addr()
        .map(|value| value.to_string())
}

fn render_signup_page(
    session: &Session,
    status: StatusCode,
    error: Option<&str>,
    current_user: Option<&users::Model>,
    current_org_name: Option<&str>,
    form: Option<&OrganisationSignupForm>,
) -> HttpResponse {
    let tera = Tera::new("../frontend/templates/**/*").expect("Failed to load templates");
    let mut context: Context = build_page_context(session);

    let already_belongs = current_user.and_then(|user| user.org_id).is_some();
    context.insert("show_admin_fields", &current_user.is_none());
    context.insert("already_belongs", &already_belongs);
    context.insert("dashboard_path", &ORG_DASHBOARD_PATH);
    if !already_belongs {
        match recaptcha_site_key() {
            Ok(site_key) => {
                context.insert("recaptcha_site_key", &site_key);
            }
            Err(message) => {
                println!("reCAPTCHA site key error: {}", message);
                return HttpResponse::InternalServerError()
                    .content_type("text/plain")
                    .body(message);
            }
        }
    }

    if let Some(error) = error {
        context.insert("error", error);
    }
    if let Ok(Some(success)) = session.get::<String>("organisation_signup_success") {
        context.insert("success", &success);
        session.remove("organisation_signup_success");
    }
    if let Some(org_name) = current_org_name {
        context.insert("current_org_name", org_name);
    }

    context.insert(
        "org_name",
        &form.map(|form| form.org_name.as_str()).unwrap_or_default(),
    );
    context.insert(
        "org_slug",
        &form.map(|form| form.org_slug.as_str()).unwrap_or_default(),
    );
    context.insert(
        "org_type",
        &form
            .and_then(|form| form.org_type.as_deref())
            .unwrap_or_default(),
    );
    context.insert(
        "website_url",
        &form
            .and_then(|form| form.website_url.as_deref())
            .unwrap_or_default(),
    );
    context.insert(
        "admin_first_name",
        &form
            .and_then(|form| form.admin_first_name.as_deref())
            .unwrap_or_default(),
    );
    context.insert(
        "admin_last_name",
        &form
            .and_then(|form| form.admin_last_name.as_deref())
            .unwrap_or_default(),
    );
    context.insert(
        "admin_email",
        &form
            .and_then(|form| form.admin_email.as_deref())
            .unwrap_or_default(),
    );

    let html = tera
        .render("organisation_signup.html", &context)
        .expect("Failed to render organisation_signup.html");

    HttpResponse::build(status)
        .content_type("text/html")
        .body(html)
}

fn validate_signup_form(
    form: &OrganisationSignupForm,
    needs_admin_account: bool,
) -> Option<&'static str> {
    if form.org_name.trim().is_empty() {
        return Some("Organisation name is required.");
    }
    if form.org_slug.trim().is_empty() {
        return Some("Organisation slug is required.");
    }

    if needs_admin_account {
        if form
            .admin_first_name
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Some("Admin first name is required.");
        }
        if form
            .admin_last_name
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Some("Admin last name is required.");
        }
        if form
            .admin_email
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Some("Admin email is required.");
        }

        let password_len = form
            .admin_password
            .as_deref()
            .unwrap_or_default()
            .chars()
            .count();
        if !(8..=128).contains(&password_len) {
            return Some("Admin password must be between 8 and 128 characters.");
        }

        let confirm_len = form
            .confirm_password
            .as_deref()
            .unwrap_or_default()
            .chars()
            .count();
        if !(8..=128).contains(&confirm_len) {
            return Some("Confirm password must be between 8 and 128 characters.");
        }

        if form.admin_password != form.confirm_password {
            return Some("Passwords do not match.");
        }
    }

    if form.validate().is_err() {
        return Some("Please check your organisation signup details.");
    }

    None
}

fn optional_trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn normalize_website_url(value: Option<&str>) -> Option<String> {
    let mut value = value?.trim();

    if value.is_empty() {
        return None;
    }

    let lower_value = value.to_ascii_lowercase();
    if lower_value.starts_with("https://") {
        value = &value[8..];
    } else if lower_value.starts_with("http://") {
        value = &value[7..];
    }

    if value.to_ascii_lowercase().starts_with("www.") {
        value = &value[4..];
    }

    Some(value.trim_end_matches('/').to_string())
}

fn generate_temp_password() -> String {
    let raw = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    format!("SkillUp-{}", &raw[..18])
}

fn title_case_ascii(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => format!(
            "{}{}",
            first.to_ascii_uppercase(),
            chars.as_str().to_ascii_lowercase()
        ),
        None => String::new(),
    }
}

fn name_from_email(email: &str) -> (String, String) {
    let local_part = email.split('@').next().unwrap_or("instructor");
    let mut parts = local_part
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(title_case_ascii)
        .collect::<Vec<String>>();

    if parts.is_empty() {
        return ("Instructor".to_string(), "User".to_string());
    }

    let first_name = parts.remove(0);
    let last_name = if parts.is_empty() {
        "Instructor".to_string()
    } else {
        parts.join(" ")
    };

    (first_name, last_name)
}

fn login_url() -> String {
    let base_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    format!("{}/login", base_url.trim_end_matches('/'))
}

fn role_label(role_name: &roles::RoleName) -> &'static str {
    match role_name {
        roles::RoleName::LmsAdmin => "LMS admin",
        roles::RoleName::OrganisationAdmin => "organisation admin",
        roles::RoleName::Instructor => "instructor",
        roles::RoleName::Student => "student",
    }
}

fn role_article(role_name: &roles::RoleName) -> &'static str {
    match role_name {
        roles::RoleName::Instructor | roles::RoleName::OrganisationAdmin => "an",
        _ => "a",
    }
}

fn send_temp_password_account_email(
    email: &str,
    temp_password: &str,
    role_name: &roles::RoleName,
) -> Result<(), String> {
    let label = role_label(role_name);
    let body = format!(
        "You have been invited as {} {} on SkillUp LMS.\n\nLogin email: {}\nTemporary password: {}\n\nSign in here: {}\n\nYou will be asked to change this temporary password after signing in.",
        role_article(role_name),
        label,
        email,
        temp_password,
        login_url()
    );

    send_mail_message(MailRequest {
        to: email.to_string(),
        subject: format!("Your SkillUp LMS {} account", label),
        body,
        is_html: false,
    })
}

struct TempPasswordAccount {
    user: users::Model,
    email: String,
    temp_password: String,
    role_name: roles::RoleName,
}

async fn create_temp_password_org_user<C>(
    db: &C,
    org_id: i32,
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    role_name: roles::RoleName,
) -> Result<TempPasswordAccount, String>
where
    C: ConnectionTrait,
{
    let temp_password = generate_temp_password();
    let password_hash = hash_password(temp_password.clone()).await?;
    let (fallback_first_name, fallback_last_name) = name_from_email(&email);

    let new_user = users::ActiveModel {
        first_name: Set(first_name
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or(fallback_first_name)),
        last_name: Set(last_name
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or(fallback_last_name)),
        email: Set(email.clone()),
        password_hash: Set(Some(password_hash)),
        auth_provider: Set("password".to_string()),
        org_id: Set(Some(org_id)),
        email_verified: Set(true),
        must_change_password: Set(true),
        ..Default::default()
    };

    let inserted_user = new_user.insert(db).await.map_err(|err| {
        format!(
            "Failed to create {} account: {}",
            role_label(&role_name),
            err
        )
    })?;

    assign_role_if_missing(db, inserted_user.user_id, &role_name)
        .await
        .map_err(|err| format!("Failed to assign {} role: {}", role_label(&role_name), err))?;

    Ok(TempPasswordAccount {
        user: inserted_user,
        email,
        temp_password,
        role_name,
    })
}

async fn assign_role_if_missing<C>(
    db: &C,
    user_id: i32,
    role_name: &roles::RoleName,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let role = roles::Entity::find()
        .filter(roles::Column::RoleName.eq(role_name.clone()))
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Role not found in database.".to_string()))?;

    let already_has_role = user_roles::Entity::find()
        .filter(user_roles::Column::UserId.eq(user_id))
        .filter(user_roles::Column::RoleId.eq(role.role_id))
        .one(db)
        .await?
        .is_some();

    if !already_has_role {
        user_roles::ActiveModel {
            user_id: Set(user_id),
            role_id: Set(role.role_id),
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

/// GET /api/organisations  –  list the current organisation admin's organisation
async fn require_session_organisation(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<i32, HttpResponse> {
    organisation_service::require_org_admin(session)?;

    if !has_role(session, "Organisation Admin") {
        return Err(HttpResponse::Forbidden().body("Organisation Admin role required"));
    }

    get_session_user_org_id(db, session).await?.ok_or_else(|| {
        HttpResponse::Forbidden().body("Organisation Admin is not assigned to an organisation")
    })
}

async fn require_matching_session_organisation(
    db: &DatabaseConnection,
    session: &Session,
    org_id: i32,
) -> Result<(), HttpResponse> {
    let session_org_id = require_session_organisation(db, session).await?;
    if session_org_id == org_id {
        Ok(())
    } else {
        Err(HttpResponse::Forbidden().body("You can only manage your organisation"))
    }
}

fn session_user_id(session: &Session) -> Result<i32, HttpResponse> {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => Ok(user_id),
        Ok(None) => Err(HttpResponse::Unauthorized().body("Please log in.")),
        Err(err) => {
            println!("Session user lookup error: {:?}", err);
            Err(HttpResponse::InternalServerError().body("Unable to read session."))
        }
    }
}

async fn instructor_role_id(db: &DatabaseConnection) -> Result<i32, HttpResponse> {
    roles::Entity::find()
        .filter(roles::Column::RoleName.eq(roles::RoleName::Instructor))
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding instructor role: {}", err))
        })?
        .map(|role| role.role_id)
        .ok_or_else(|| HttpResponse::InternalServerError().body("Instructor role not configured"))
}

async fn user_has_role_id(
    db: &DatabaseConnection,
    user_id: i32,
    role_id: i32,
) -> Result<bool, HttpResponse> {
    user_roles::Entity::find()
        .filter(user_roles::Column::UserId.eq(user_id))
        .filter(user_roles::Column::RoleId.eq(role_id))
        .one(db)
        .await
        .map(|role| role.is_some())
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error checking user role: {}", err))
        })
}

async fn lms_admin_user_ids<C>(db: &C) -> Result<HashSet<i32>, DbErr>
where
    C: ConnectionTrait,
{
    let Some(lms_admin_role) = roles::Entity::find()
        .filter(roles::Column::RoleName.eq(roles::RoleName::LmsAdmin))
        .one(db)
        .await?
    else {
        return Ok(HashSet::new());
    };

    let rows = user_roles::Entity::find()
        .filter(user_roles::Column::RoleId.eq(lms_admin_role.role_id))
        .all(db)
        .await?;

    Ok(rows.into_iter().map(|row| row.user_id).collect())
}

async fn assign_role_id_if_missing<C>(db: &C, user_id: i32, role_id: i32) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let already_has_role = user_roles::Entity::find()
        .filter(user_roles::Column::UserId.eq(user_id))
        .filter(user_roles::Column::RoleId.eq(role_id))
        .one(db)
        .await?
        .is_some();

    if !already_has_role {
        user_roles::ActiveModel {
            user_id: Set(user_id),
            role_id: Set(role_id),
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

async fn attach_existing_user_to_organisation<C>(
    db: &C,
    user: users::Model,
    org_id: i32,
    role_id: i32,
    lms_admins: &HashSet<i32>,
) -> Result<i32, String>
where
    C: ConnectionTrait,
{
    let user_id = user.user_id;

    if lms_admins.contains(&user_id) {
        return Err(format!(
            "User {} is an LMS admin and cannot be added",
            user_id
        ));
    }

    if let Some(existing_org_id) = user.org_id {
        if existing_org_id == org_id {
            return Err(format!(
                "User {} already belongs to this organisation",
                user_id
            ));
        }

        return Err(format!(
            "User {} already belongs to another organisation",
            user_id
        ));
    }

    let mut active_user = sea_orm::IntoActiveModel::into_active_model(user);
    active_user.org_id = Set(Some(org_id));
    active_user
        .update(db)
        .await
        .map_err(|err| format!("Failed to set org for user {}: {}", user_id, err))?;

    assign_role_id_if_missing(db, user_id, role_id)
        .await
        .map_err(|err| format!("Failed to assign role for user {}: {}", user_id, err))?;

    Ok(user_id)
}

async fn require_course_in_organisation(
    db: &DatabaseConnection,
    course_id: i32,
    org_id: i32,
) -> Result<courses::Model, HttpResponse> {
    let course = courses::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    if course.org_id == Some(org_id) {
        Ok(course)
    } else {
        Err(HttpResponse::Forbidden().body("Course is not in your organisation"))
    }
}

async fn require_instructor_in_organisation(
    db: &DatabaseConnection,
    instructor_id: i32,
    org_id: i32,
) -> Result<users::Model, HttpResponse> {
    let instructor = users::Entity::find_by_id(instructor_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding instructor: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Instructor not found"))?;

    if instructor.org_id != Some(org_id) {
        return Err(HttpResponse::Forbidden().body("Instructor is not in your organisation"));
    }

    let role_id = instructor_role_id(db).await?;
    if !user_has_role_id(db, instructor_id, role_id).await? {
        return Err(HttpResponse::BadRequest().body("User is not an instructor"));
    }

    Ok(instructor)
}

#[get("/organisations")]
pub async fn list_organisations(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let org_id = match require_session_organisation(db.get_ref(), &session).await {
        Ok(org_id) => org_id,
        Err(response) => return response,
    };

    match organisations::Entity::find_by_id(org_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(org)) => HttpResponse::Ok().json(vec![org]),
        Ok(None) => HttpResponse::NotFound().body("Organisation not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

/// GET /api/organisations/{org_id}  –  single organisation
#[get("/organisations/{org_id}")]
pub async fn get_organisation(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let org_id = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    match organisations::Entity::find_by_id(org_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(org)) => HttpResponse::Ok().json(org),
        Ok(None) => HttpResponse::NotFound().body("Organisation not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

/// POST /api/organisations  –  create organisation (admin only)
#[post("/organisations")]
pub async fn create_organisation(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateOrganisationForm>,
) -> impl Responder {
    if !has_role(&session, "LMS Admin") {
        return HttpResponse::Forbidden().body("LMS Admin role required");
    }

    match current_session_user(db.get_ref(), &session).await {
        Ok(Some(user)) if user.email_verified => {}
        Ok(Some(_)) => {
            return HttpResponse::Forbidden()
                .body("Please verify your email before creating an organisation.");
        }
        Ok(None) => return HttpResponse::Unauthorized().body("Please log in."),
        Err(response) => return response,
    }

    let new_org = organisations::ActiveModel {
        org_name: Set(body.org_name.trim().to_string()),
        ..Default::default()
    };

    match new_org.insert(db.get_ref()).await {
        Ok(org) => HttpResponse::Ok().json(org),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to create organisation: {}", err)),
    }
}

/// DELETE /api/organisations/{org_id}  –  delete organisation (admin only)
#[delete("/organisations/{org_id}")]
pub async fn delete_organisation(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let org_id = path.into_inner();
    if !has_role(&session, "LMS Admin") {
        return HttpResponse::Forbidden().body("LMS Admin role required");
    }

    organisation_service::delete_organisation_and_dependents(db.get_ref(), org_id).await
}

// ── Members ────────────────────────────────────────────────────────────────────

/// GET /api/organisations/{org_id}/members  –  list all members with their roles
#[get("/organisations/{org_id}/members")]
pub async fn list_org_members(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let org_id = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    let members = match users::Entity::find()
        .filter(users::Column::OrgId.eq(org_id))
        .all(db.get_ref())
        .await
    {
        Ok(m) => m,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error fetching members: {}", err));
        }
    };

    // fetch all roles (small table – load once)
    let all_roles = match roles::Entity::find().all(db.get_ref()).await {
        Ok(r) => r,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error fetching roles: {}", err));
        }
    };

    let mut dtos: Vec<OrgMemberDto> = Vec::new();

    for user in members {
        let user_role_rows = match user_roles::Entity::find()
            .filter(user_roles::Column::UserId.eq(user.user_id))
            .all(db.get_ref())
            .await
        {
            Ok(rows) => rows,
            Err(_) => vec![],
        };

        // Use the sea_orm string value instead of Debug
        let role_names: Vec<String> = user_role_rows
            .iter()
            .filter_map(|ur| {
                all_roles
                    .iter()
                    .find(|r| r.role_id == ur.role_id)
                    .map(|r| match r.role_name {
                        roles::RoleName::LmsAdmin => "LMS Admin".to_string(),
                        roles::RoleName::OrganisationAdmin => "Organisation Admin".to_string(),
                        roles::RoleName::Instructor => "Instructor".to_string(),
                        roles::RoleName::Student => "Student".to_string(),
                    })
            })
            .collect();

        dtos.push(OrgMemberDto {
            user_id: user.user_id,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            roles: role_names,
        });
    }

    HttpResponse::Ok().json(dtos)
}

/// POST /api/organisations/{org_id}/instructors/invite
#[post("/organisations/{org_id}/instructors/invite")]
pub async fn invite_instructor(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<InviteInstructorForm>,
) -> impl Responder {
    let org_id = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    let body = body.into_inner();
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest().body(format!("Validation error: {}", errors));
    }

    let email = body.email.trim().to_lowercase();
    match users::Entity::find()
        .filter(users::Column::Email.eq(email.clone()))
        .one(db.get_ref())
        .await
    {
        Ok(Some(_)) => return HttpResponse::Conflict().body("Email is already registered"),
        Ok(None) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error checking email: {}", err));
        }
    }

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err));
        }
    };

    let invited_account = match create_temp_password_org_user(
        &txn,
        org_id,
        email.clone(),
        None,
        None,
        roles::RoleName::Instructor,
    )
    .await
    {
        Ok(account) => account,
        Err(message) => return HttpResponse::InternalServerError().body(message),
    };

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to commit instructor invite: {}", err));
    }

    match send_temp_password_account_email(
        &invited_account.email,
        &invited_account.temp_password,
        &invited_account.role_name,
    ) {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "user_id": invited_account.user.user_id,
            "email": invited_account.email,
            "message": "Instructor invited successfully"
        })),
        Err(err) => HttpResponse::InternalServerError().body(format!(
            "Instructor account created, but the invite email could not be sent: {}",
            err
        )),
    }
}

// ── Mass enrollment ────────────────────────────────────────────────────────────

fn normalize_class_name(value: &str) -> Result<String, HttpResponse> {
    let name = value.trim().to_string();
    if name.is_empty() {
        return Err(HttpResponse::BadRequest().body("Class name is required"));
    }
    if name.chars().count() > 255 {
        return Err(HttpResponse::BadRequest().body("Class name must be 255 characters or fewer"));
    }
    Ok(name)
}

fn class_name_key(value: &str) -> String {
    value.trim().to_lowercase()
}

async fn validate_class_course_ids(
    db: &DatabaseConnection,
    org_id: i32,
    course_ids: Vec<i32>,
) -> Result<Vec<i32>, HttpResponse> {
    let mut deduped = Vec::new();
    let mut seen = HashSet::new();

    for course_id in course_ids {
        if course_id <= 0 {
            return Err(HttpResponse::BadRequest().body("Course IDs must be valid records"));
        }
        if seen.insert(course_id) {
            deduped.push(course_id);
        }
    }

    if deduped.is_empty() {
        return Err(HttpResponse::BadRequest().body("Select at least one course for the class"));
    }

    for &course_id in &deduped {
        require_course_in_organisation(db, course_id, org_id).await?;
    }

    Ok(deduped)
}

async fn student_role_id<C>(db: &C) -> Result<i32, DbErr>
where
    C: ConnectionTrait,
{
    roles::Entity::find()
        .filter(roles::Column::RoleName.eq(roles::RoleName::Student))
        .one(db)
        .await?
        .map(|role| role.role_id)
        .ok_or_else(|| DbErr::RecordNotFound("Student role not found in database".to_string()))
}

async fn require_class_in_organisation(
    db: &DatabaseConnection,
    class_id: i32,
    org_id: i32,
) -> Result<org_classes::Model, HttpResponse> {
    let class = org_classes::Entity::find_by_id(class_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding class: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Class not found"))?;

    if class.org_id == org_id {
        Ok(class)
    } else {
        Err(HttpResponse::Forbidden().body("Class is not in your organisation"))
    }
}

async fn ensure_course_enrollment<C>(db: &C, user_id: i32, course_id: i32) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let exists = enrollments::Entity::find_by_id((user_id, course_id))
        .one(db)
        .await?
        .is_some();

    if !exists {
        enrollments::ActiveModel {
            user_id: Set(user_id),
            course_id: Set(course_id),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

async fn ensure_class_membership<C>(
    db: &C,
    class_id: i32,
    user_id: i32,
    assigned_by: i32,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let exists = org_class_members::Entity::find_by_id((class_id, user_id))
        .one(db)
        .await?
        .is_some();

    if !exists {
        org_class_members::ActiveModel {
            class_id: Set(class_id),
            user_id: Set(user_id),
            assigned_by: Set(Some(assigned_by)),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

async fn class_course_ids<C>(db: &C, class_id: i32) -> Result<Vec<i32>, DbErr>
where
    C: ConnectionTrait,
{
    org_class_courses::Entity::find()
        .filter(org_class_courses::Column::ClassId.eq(class_id))
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(|row| row.course_id).collect())
}

async fn set_class_courses<C>(db: &C, class_id: i32, course_ids: &[i32]) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    org_class_courses::Entity::delete_many()
        .filter(org_class_courses::Column::ClassId.eq(class_id))
        .exec(db)
        .await?;

    for &course_id in course_ids {
        org_class_courses::ActiveModel {
            class_id: Set(class_id),
            course_id: Set(course_id),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

async fn has_other_class_for_course<C>(
    db: &C,
    user_id: i32,
    excluded_class_id: i32,
    course_id: i32,
) -> Result<bool, DbErr>
where
    C: ConnectionTrait,
{
    let same_course_classes = org_class_courses::Entity::find()
        .filter(org_class_courses::Column::CourseId.eq(course_id))
        .filter(org_class_courses::Column::ClassId.ne(excluded_class_id))
        .all(db)
        .await?;

    let class_ids = same_course_classes
        .into_iter()
        .map(|class_course| class_course.class_id)
        .collect::<Vec<i32>>();

    if class_ids.is_empty() {
        return Ok(false);
    }

    org_class_members::Entity::find()
        .filter(org_class_members::Column::UserId.eq(user_id))
        .filter(org_class_members::Column::ClassId.is_in(class_ids))
        .one(db)
        .await
        .map(|row| row.is_some())
}

async fn remove_course_enrollment_if_unassigned<C>(
    db: &C,
    user_id: i32,
    excluded_class_id: i32,
    course_id: i32,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if has_other_class_for_course(db, user_id, excluded_class_id, course_id).await? {
        return Ok(());
    }

    enrollments::Entity::delete_by_id((user_id, course_id))
        .exec(db)
        .await?;

    Ok(())
}

async fn ensure_existing_user_for_class<C>(
    db: &C,
    user: users::Model,
    org_id: i32,
    student_role_id: i32,
    lms_admins: &HashSet<i32>,
) -> Result<i32, String>
where
    C: ConnectionTrait,
{
    let user_id = user.user_id;

    if lms_admins.contains(&user_id) {
        return Err(format!(
            "User {} is an LMS admin and cannot be added",
            user_id
        ));
    }

    match user.org_id {
        Some(existing_org_id) if existing_org_id == org_id => {}
        Some(_) => {
            return Err(format!(
                "User {} already belongs to another organisation",
                user_id
            ));
        }
        None => {
            let mut active_user = sea_orm::IntoActiveModel::into_active_model(user);
            active_user.org_id = Set(Some(org_id));
            active_user
                .update(db)
                .await
                .map_err(|err| format!("Failed to set org for user {}: {}", user_id, err))?;
        }
    }

    assign_role_id_if_missing(db, user_id, student_role_id)
        .await
        .map_err(|err| {
            format!(
                "Failed to assign student role for user {}: {}",
                user_id, err
            )
        })?;

    Ok(user_id)
}

async fn add_user_to_class<C>(
    db: &C,
    class: &org_classes::Model,
    user_id: i32,
    assigned_by: i32,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    ensure_class_membership(db, class.class_id, user_id, assigned_by)
        .await
        .map_err(|err| format!("Failed to add user {} to class: {}", user_id, err))?;

    let course_ids = class_course_ids(db, class.class_id)
        .await
        .map_err(|err| format!("Failed to find class courses: {}", err))?;

    for course_id in course_ids {
        ensure_course_enrollment(db, user_id, course_id)
            .await
            .map_err(|err| {
                format!(
                    "Failed to enroll user {} into class course: {}",
                    user_id, err
                )
            })?;
    }

    Ok(())
}

async fn sync_class_courses_change<C>(
    db: &C,
    class_id: i32,
    old_course_ids: &[i32],
    new_course_ids: &[i32],
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let members = org_class_members::Entity::find()
        .filter(org_class_members::Column::ClassId.eq(class_id))
        .all(db)
        .await?;

    let old_courses = old_course_ids.iter().copied().collect::<HashSet<i32>>();
    let new_courses = new_course_ids.iter().copied().collect::<HashSet<i32>>();
    let removed_courses = old_courses
        .difference(&new_courses)
        .copied()
        .collect::<Vec<i32>>();
    let added_courses = new_courses
        .difference(&old_courses)
        .copied()
        .collect::<Vec<i32>>();

    for member in members {
        for course_id in &removed_courses {
            remove_course_enrollment_if_unassigned(db, member.user_id, class_id, *course_id)
                .await?;
        }
        for course_id in &added_courses {
            ensure_course_enrollment(db, member.user_id, *course_id).await?;
        }
    }

    Ok(())
}

fn org_class_member_dto(user: users::Model) -> OrgClassMemberDto {
    OrgClassMemberDto {
        user_id: user.user_id,
        first_name: user.first_name,
        last_name: user.last_name,
        email: user.email,
    }
}

async fn add_class_members_impl<C>(
    db: &C,
    class: &org_classes::Model,
    body: &AddClassMembersForm,
    assigned_by: i32,
) -> Result<(Vec<i32>, Vec<TempPasswordAccount>, Vec<String>), String>
where
    C: ConnectionTrait,
{
    let role_id = student_role_id(db)
        .await
        .map_err(|err| format!("Failed to find student role: {}", err))?;
    let lms_admins = lms_admin_user_ids(db)
        .await
        .map_err(|err| format!("Failed to check LMS admin users: {}", err))?;

    let mut added = Vec::new();
    let mut errors = Vec::new();
    let mut created_accounts = Vec::new();
    let mut seen_new_emails = HashSet::new();

    for &user_id in &body.user_ids {
        let user = match users::Entity::find_by_id(user_id).one(db).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                errors.push(format!("User {} not found", user_id));
                continue;
            }
            Err(err) => {
                errors.push(format!("Database error finding user {}: {}", user_id, err));
                continue;
            }
        };

        match ensure_existing_user_for_class(db, user, class.org_id, role_id, &lms_admins).await {
            Ok(user_id) => match add_user_to_class(db, class, user_id, assigned_by).await {
                Ok(()) => added.push(user_id),
                Err(message) => errors.push(message),
            },
            Err(message) => errors.push(message),
        }
    }

    for new_user in &body.new_users {
        let email = new_user.email.trim().to_lowercase();
        if email.is_empty() {
            errors.push("Skipped a new user row with a blank email".to_string());
            continue;
        }
        if !validate_email(&email) {
            errors.push(format!("{} is not a valid email address", email));
            continue;
        }
        if !seen_new_emails.insert(email.clone()) {
            errors.push(format!("{} appears more than once in the import", email));
            continue;
        }

        match users::Entity::find()
            .filter(users::Column::Email.eq(email.clone()))
            .one(db)
            .await
        {
            Ok(Some(existing_user)) => {
                match ensure_existing_user_for_class(
                    db,
                    existing_user,
                    class.org_id,
                    role_id,
                    &lms_admins,
                )
                .await
                {
                    Ok(user_id) => match add_user_to_class(db, class, user_id, assigned_by).await {
                        Ok(()) => added.push(user_id),
                        Err(message) => errors.push(format!("{}: {}", email, message)),
                    },
                    Err(message) => errors.push(format!("{}: {}", email, message)),
                }
            }
            Ok(None) => {
                match create_temp_password_org_user(
                    db,
                    class.org_id,
                    email.clone(),
                    new_user.first_name.clone(),
                    new_user.last_name.clone(),
                    roles::RoleName::Student,
                )
                .await
                {
                    Ok(account) => {
                        match add_user_to_class(db, class, account.user.user_id, assigned_by).await
                        {
                            Ok(()) => {
                                added.push(account.user.user_id);
                                created_accounts.push(account);
                            }
                            Err(message) => errors.push(format!("{}: {}", email, message)),
                        }
                    }
                    Err(message) => errors.push(format!("{}: {}", email, message)),
                }
            }
            Err(err) => errors.push(format!("Database error checking {}: {}", email, err)),
        }
    }

    Ok((added, created_accounts, errors))
}

/// GET /api/organisations/{org_id}/classes
#[get("/organisations/{org_id}/classes")]
pub async fn list_org_classes(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let org_id = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    let course_rows = match courses::Entity::find()
        .filter(courses::Column::OrgId.eq(org_id))
        .order_by_asc(courses::Column::Name)
        .all(db.get_ref())
        .await
    {
        Ok(courses) => courses,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding courses: {}", err));
        }
    };

    let courses_by_id = course_rows
        .iter()
        .map(|course| {
            (
                course.course_id,
                course
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Course #{}", course.course_id)),
            )
        })
        .collect::<HashMap<i32, String>>();

    let class_rows = match org_classes::Entity::find()
        .filter(org_classes::Column::OrgId.eq(org_id))
        .order_by_asc(org_classes::Column::ClassName)
        .all(db.get_ref())
        .await
    {
        Ok(classes) => classes,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding classes: {}", err));
        }
    };

    let class_ids = class_rows
        .iter()
        .map(|class| class.class_id)
        .collect::<Vec<i32>>();

    let class_course_rows = if class_ids.is_empty() {
        Vec::new()
    } else {
        match org_class_courses::Entity::find()
            .filter(org_class_courses::Column::ClassId.is_in(class_ids.clone()))
            .all(db.get_ref())
            .await
        {
            Ok(class_courses) => class_courses,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding class courses: {}", err));
            }
        }
    };

    let mut courses_by_class: HashMap<i32, Vec<OrgClassCourseDto>> = HashMap::new();
    for class_course in class_course_rows {
        courses_by_class
            .entry(class_course.class_id)
            .or_default()
            .push(OrgClassCourseDto {
                course_id: class_course.course_id,
                name: courses_by_id
                    .get(&class_course.course_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Course #{}", class_course.course_id)),
            });
    }
    for class_courses in courses_by_class.values_mut() {
        class_courses.sort_by(|a, b| a.name.cmp(&b.name));
    }

    let member_rows = if class_ids.is_empty() {
        Vec::new()
    } else {
        match org_class_members::Entity::find()
            .filter(org_class_members::Column::ClassId.is_in(class_ids.clone()))
            .all(db.get_ref())
            .await
        {
            Ok(members) => members,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding class members: {}", err));
            }
        }
    };

    let user_ids = member_rows
        .iter()
        .map(|member| member.user_id)
        .collect::<HashSet<i32>>();

    let users_by_id = if user_ids.is_empty() {
        HashMap::new()
    } else {
        match users::Entity::find()
            .filter(users::Column::UserId.is_in(user_ids.iter().copied()))
            .all(db.get_ref())
            .await
        {
            Ok(users) => users
                .into_iter()
                .map(|user| (user.user_id, org_class_member_dto(user)))
                .collect::<HashMap<i32, OrgClassMemberDto>>(),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding class users: {}", err));
            }
        }
    };

    let mut members_by_class: HashMap<i32, Vec<OrgClassMemberDto>> = HashMap::new();
    for member in member_rows {
        if let Some(user) = users_by_id.get(&member.user_id) {
            members_by_class
                .entry(member.class_id)
                .or_default()
                .push(user.clone());
        }
    }
    for members in members_by_class.values_mut() {
        members.sort_by(|a, b| {
            a.last_name
                .cmp(&b.last_name)
                .then_with(|| a.first_name.cmp(&b.first_name))
                .then_with(|| a.email.cmp(&b.email))
        });
    }

    let class_dtos = class_rows
        .into_iter()
        .map(|class| OrgClassDto {
            class_id: class.class_id,
            org_id: class.org_id,
            courses: courses_by_class.remove(&class.class_id).unwrap_or_default(),
            class_name: class.class_name,
            members: members_by_class.remove(&class.class_id).unwrap_or_default(),
        })
        .collect::<Vec<OrgClassDto>>();

    let course_dtos = course_rows
        .into_iter()
        .map(|course| OrgClassCourseDto {
            course_id: course.course_id,
            name: course
                .name
                .unwrap_or_else(|| format!("Course #{}", course.course_id)),
        })
        .collect::<Vec<OrgClassCourseDto>>();

    HttpResponse::Ok().json(OrgClassSummaryDto {
        classes: class_dtos,
        courses: course_dtos,
    })
}

/// POST /api/organisations/{org_id}/classes
#[post("/organisations/{org_id}/classes")]
pub async fn create_org_class(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<CreateOrgClassForm>,
) -> impl Responder {
    let org_id = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    let body = body.into_inner();
    let class_name = match normalize_class_name(&body.class_name) {
        Ok(name) => name,
        Err(response) => return response,
    };
    let course_ids = match validate_class_course_ids(db.get_ref(), org_id, body.course_ids).await {
        Ok(course_ids) => course_ids,
        Err(response) => return response,
    };

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err));
        }
    };

    let class = org_classes::ActiveModel {
        org_id: Set(org_id),
        class_name: Set(class_name),
        ..Default::default()
    };

    match class.insert(&txn).await {
        Ok(saved) => {
            if let Err(err) = set_class_courses(&txn, saved.class_id, &course_ids).await {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to assign class courses: {}", err));
            }
            if let Err(err) = txn.commit().await {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to commit class creation: {}", err));
            }
            HttpResponse::Ok().json(saved)
        }
        Err(DbErr::Exec(_)) => HttpResponse::Conflict().body("Class name already exists"),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to create class: {}", err))
        }
    }
}

/// PUT /api/organisations/{org_id}/classes/{class_id}
#[put("/organisations/{org_id}/classes/{class_id}")]
pub async fn update_org_class(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32)>,
    body: web::Json<UpdateOrgClassForm>,
) -> impl Responder {
    let (org_id, class_id) = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    let class = match require_class_in_organisation(db.get_ref(), class_id, org_id).await {
        Ok(class) => class,
        Err(response) => return response,
    };
    let body = body.into_inner();
    let new_course_ids = match body.course_ids {
        Some(course_ids) => match validate_class_course_ids(db.get_ref(), org_id, course_ids).await
        {
            Ok(course_ids) => Some(course_ids),
            Err(response) => return response,
        },
        None => None,
    };

    let new_name = match body.class_name {
        Some(name) => match normalize_class_name(&name) {
            Ok(name) => Some(name),
            Err(response) => return response,
        },
        None => None,
    };

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err));
        }
    };

    let old_course_ids = match class_course_ids(&txn, class.class_id).await {
        Ok(course_ids) => course_ids,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to find current class courses: {}", err));
        }
    };

    let mut active = sea_orm::IntoActiveModel::into_active_model(class.clone());
    if let Some(name) = new_name {
        active.class_name = Set(name);
    }

    match active.update(&txn).await {
        Ok(saved) => {
            if let Some(new_course_ids) = new_course_ids.as_ref() {
                if let Err(err) = set_class_courses(&txn, class.class_id, new_course_ids).await {
                    return HttpResponse::InternalServerError()
                        .body(format!("Failed to update class courses: {}", err));
                }
                if let Err(err) =
                    sync_class_courses_change(&txn, class.class_id, &old_course_ids, new_course_ids)
                        .await
                {
                    return HttpResponse::InternalServerError()
                        .body(format!("Failed to sync class enrollments: {}", err));
                }
            }
            if let Err(err) = txn.commit().await {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to commit class update: {}", err));
            }
            HttpResponse::Ok().json(saved)
        }
        Err(DbErr::Exec(_)) => HttpResponse::Conflict().body("Class name already exists"),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to update class: {}", err))
        }
    }
}

/// DELETE /api/organisations/{org_id}/classes/{class_id}
#[delete("/organisations/{org_id}/classes/{class_id}")]
pub async fn delete_org_class(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32)>,
) -> impl Responder {
    let (org_id, class_id) = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    let _class = match require_class_in_organisation(db.get_ref(), class_id, org_id).await {
        Ok(class) => class,
        Err(response) => return response,
    };

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err));
        }
    };

    let course_ids = match class_course_ids(&txn, class_id).await {
        Ok(course_ids) => course_ids,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to find class courses: {}", err));
        }
    };

    let members = match org_class_members::Entity::find()
        .filter(org_class_members::Column::ClassId.eq(class_id))
        .all(&txn)
        .await
    {
        Ok(members) => members,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding class members: {}", err));
        }
    };

    if let Err(err) = org_class_members::Entity::delete_many()
        .filter(org_class_members::Column::ClassId.eq(class_id))
        .exec(&txn)
        .await
    {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to remove class members: {}", err));
    }

    for member in members {
        for course_id in &course_ids {
            if let Err(err) =
                remove_course_enrollment_if_unassigned(&txn, member.user_id, class_id, *course_id)
                    .await
            {
                return HttpResponse::InternalServerError().body(format!(
                    "Failed to sync enrollment for user {}: {}",
                    member.user_id, err
                ));
            }
        }
    }

    if let Err(err) = org_classes::Entity::delete_by_id(class_id).exec(&txn).await {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to delete class: {}", err));
    }

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to commit class delete: {}", err));
    }

    HttpResponse::Ok().json(serde_json::json!({ "message": "Class deleted" }))
}

/// POST /api/organisations/{org_id}/classes/{class_id}/members
#[post("/organisations/{org_id}/classes/{class_id}/members")]
pub async fn add_org_class_members(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32)>,
    body: web::Json<AddClassMembersForm>,
) -> impl Responder {
    let (org_id, class_id) = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }
    let assigned_by = match session_user_id(&session) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    let class = match require_class_in_organisation(db.get_ref(), class_id, org_id).await {
        Ok(class) => class,
        Err(response) => return response,
    };
    let body = body.into_inner();

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err));
        }
    };

    let (added, created_accounts, mut errors) =
        match add_class_members_impl(&txn, &class, &body, assigned_by).await {
            Ok(result) => result,
            Err(message) => return HttpResponse::InternalServerError().body(message),
        };

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError().body(format!(
            "Failed to commit class membership changes: {}",
            err
        ));
    }

    for account in &created_accounts {
        if let Err(err) = send_temp_password_account_email(
            &account.email,
            &account.temp_password,
            &account.role_name,
        ) {
            errors.push(format!(
                "{} account created, but the invite email could not be sent: {}",
                account.email, err
            ));
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "added": added,
        "created": created_accounts
            .iter()
            .map(|account| serde_json::json!({
                "user_id": account.user.user_id,
                "email": account.email,
            }))
            .collect::<Vec<serde_json::Value>>(),
        "errors": errors,
        "message": format!("{} learner(s) added to class", added.len())
    }))
}

/// DELETE /api/organisations/{org_id}/classes/{class_id}/members/{user_id}
#[delete("/organisations/{org_id}/classes/{class_id}/members/{user_id}")]
pub async fn remove_org_class_member(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32, i32)>,
) -> impl Responder {
    let (org_id, class_id, user_id) = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    let _class = match require_class_in_organisation(db.get_ref(), class_id, org_id).await {
        Ok(class) => class,
        Err(response) => return response,
    };

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err));
        }
    };

    let course_ids = match class_course_ids(&txn, class_id).await {
        Ok(course_ids) => course_ids,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to find class courses: {}", err));
        }
    };

    match org_class_members::Entity::delete_by_id((class_id, user_id))
        .exec(&txn)
        .await
    {
        Ok(result) if result.rows_affected > 0 => {}
        Ok(_) => return HttpResponse::NotFound().body("Class member not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to remove class member: {}", err));
        }
    }

    for course_id in course_ids {
        if let Err(err) =
            remove_course_enrollment_if_unassigned(&txn, user_id, class_id, course_id).await
        {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to sync course enrollment: {}", err));
        }
    }

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError().body(format!(
            "Failed to commit class membership removal: {}",
            err
        ));
    }

    HttpResponse::Ok().json(serde_json::json!({ "message": "Learner removed from class" }))
}

/// POST /api/organisations/{org_id}/classes/import
#[post("/organisations/{org_id}/classes/import")]
pub async fn import_org_class_members(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<ImportClassMembersForm>,
) -> impl Responder {
    let org_id = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }
    let assigned_by = match session_user_id(&session) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };

    let body = body.into_inner();
    if body.rows.is_empty() {
        return HttpResponse::BadRequest().body("No import rows provided");
    }

    let classes = match org_classes::Entity::find()
        .filter(org_classes::Column::OrgId.eq(org_id))
        .all(db.get_ref())
        .await
    {
        Ok(classes) => classes,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding classes: {}", err));
        }
    };
    let mut classes_by_name = classes
        .into_iter()
        .map(|class| (class_name_key(&class.class_name), class))
        .collect::<HashMap<String, org_classes::Model>>();

    // Collect unique new class names that don't yet exist in this org
    let mut new_class_map: HashMap<String, String> = HashMap::new();
    for row in &body.rows {
        let name = row.class_name.trim().to_string();
        let key = class_name_key(&name);
        if !name.is_empty() && !classes_by_name.contains_key(&key) {
            new_class_map.entry(key).or_insert(name);
        }
    }

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err));
        }
    };

    // Create any new classes inside the transaction
    let mut errors: Vec<String> = Vec::new();
    let mut created_class_count = 0usize;
    for (key, name) in new_class_map {
        match (org_classes::ActiveModel {
            org_id: Set(org_id),
            class_name: Set(name.clone()),
            ..Default::default()
        })
        .insert(&txn)
        .await
        {
            Ok(new_class) => {
                classes_by_name.insert(key, new_class);
                created_class_count += 1;
            }
            Err(DbErr::Exec(_)) => {
                // Race condition: another request created this class concurrently; find it
                match org_classes::Entity::find()
                    .filter(org_classes::Column::OrgId.eq(org_id))
                    .filter(org_classes::Column::ClassName.eq(name.clone()))
                    .one(&txn)
                    .await
                {
                    Ok(Some(existing)) => {
                        classes_by_name.insert(key, existing);
                    }
                    _ => {
                        errors.push(format!("Could not create or find class '{}'", name));
                    }
                }
            }
            Err(err) => {
                errors.push(format!("Failed to create class '{}': {}", name, err));
            }
        }
    }

    let mut rows_by_class: HashMap<i32, AddClassMembersForm> = HashMap::new();

    for row in body.rows {
        let email = row.email.trim().to_lowercase();
        let class_key = class_name_key(&row.class_name);
        if email.is_empty() {
            errors.push("Skipped a row with a blank email".to_string());
            continue;
        }
        if !validate_email(&email) {
            errors.push(format!("{} is not a valid email address", email));
            continue;
        }
        let Some(class) = classes_by_name.get(&class_key) else {
            errors.push(format!(
                "{}: class '{}' could not be created or found",
                email,
                row.class_name.trim()
            ));
            continue;
        };

        rows_by_class
            .entry(class.class_id)
            .or_insert_with(|| AddClassMembersForm {
                user_ids: Vec::new(),
                new_users: Vec::new(),
            })
            .new_users
            .push(crate::models::organisation::MassEnrollNewUserForm {
                email,
                first_name: row.first_name,
                last_name: row.last_name,
            });
    }

    let mut added = Vec::new();
    let mut created_accounts = Vec::new();
    for (class_id, form) in rows_by_class {
        let Some(class) = classes_by_name
            .values()
            .find(|class| class.class_id == class_id)
        else {
            continue;
        };
        match add_class_members_impl(&txn, class, &form, assigned_by).await {
            Ok((class_added, class_created, class_errors)) => {
                added.extend(class_added);
                created_accounts.extend(class_created);
                errors.extend(class_errors);
            }
            Err(message) => return HttpResponse::InternalServerError().body(message),
        }
    }

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to commit class import: {}", err));
    }

    for account in &created_accounts {
        if let Err(err) = send_temp_password_account_email(
            &account.email,
            &account.temp_password,
            &account.role_name,
        ) {
            errors.push(format!(
                "{} account created, but the invite email could not be sent: {}",
                account.email, err
            ));
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "added": added,
        "created": created_accounts
            .iter()
            .map(|account| serde_json::json!({
                "user_id": account.user.user_id,
                "email": account.email,
            }))
            .collect::<Vec<serde_json::Value>>(),
        "errors": errors,
        "message": format!(
            "{} learner(s) imported{}",
            added.len(),
            if created_class_count > 0 {
                format!(", {} new class(es) created", created_class_count)
            } else {
                String::new()
            }
        )
    }))
}

/// GET /api/organisations/{org_id}/course-instructors
#[get("/organisations/{org_id}/course-instructors")]
pub async fn list_course_instructors(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let org_id = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    let org_courses = match courses::Entity::find()
        .filter(courses::Column::OrgId.eq(org_id))
        .all(db.get_ref())
        .await
    {
        Ok(courses) => courses,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding courses: {}", err));
        }
    };

    let instructor_role_id = match instructor_role_id(db.get_ref()).await {
        Ok(role_id) => role_id,
        Err(response) => return response,
    };

    let instructor_user_ids = match user_roles::Entity::find()
        .filter(user_roles::Column::RoleId.eq(instructor_role_id))
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|row| row.user_id)
            .collect::<HashSet<i32>>(),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding instructor roles: {}", err));
        }
    };

    let org_instructors = if instructor_user_ids.is_empty() {
        Vec::new()
    } else {
        match users::Entity::find()
            .filter(users::Column::OrgId.eq(org_id))
            .filter(users::Column::UserId.is_in(instructor_user_ids.iter().copied()))
            .all(db.get_ref())
            .await
        {
            Ok(users) => users,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding instructors: {}", err));
            }
        }
    };

    let instructor_dtos = org_instructors
        .into_iter()
        .map(|user| CourseInstructorDto {
            user_id: user.user_id,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
        })
        .collect::<Vec<CourseInstructorDto>>();

    let instructor_by_id = instructor_dtos
        .iter()
        .cloned()
        .map(|instructor| (instructor.user_id, instructor))
        .collect::<HashMap<i32, CourseInstructorDto>>();

    let course_ids = org_courses
        .iter()
        .map(|course| course.course_id)
        .collect::<Vec<i32>>();

    let assignments = if course_ids.is_empty() {
        Vec::new()
    } else {
        match course_instructors::Entity::find()
            .filter(course_instructors::Column::CourseId.is_in(course_ids.clone()))
            .all(db.get_ref())
            .await
        {
            Ok(assignments) => assignments,
            Err(err) => {
                return HttpResponse::InternalServerError().body(format!(
                    "Database error finding course instructors: {}",
                    err
                ));
            }
        }
    };

    let mut assignments_by_course: HashMap<i32, Vec<CourseInstructorDto>> = HashMap::new();
    for assignment in assignments {
        if let Some(instructor) = instructor_by_id.get(&assignment.instructor_id) {
            assignments_by_course
                .entry(assignment.course_id)
                .or_default()
                .push(instructor.clone());
        }
    }

    let course_dtos = org_courses
        .into_iter()
        .map(|course| CourseInstructorCourseDto {
            course_id: course.course_id,
            name: course
                .name
                .unwrap_or_else(|| format!("Course #{}", course.course_id)),
            instructors: assignments_by_course
                .remove(&course.course_id)
                .unwrap_or_default(),
        })
        .collect::<Vec<CourseInstructorCourseDto>>();

    HttpResponse::Ok().json(CourseInstructorSummaryDto {
        courses: course_dtos,
        instructors: instructor_dtos,
    })
}

/// POST /api/organisations/{org_id}/courses/{course_id}/instructors
#[post("/organisations/{org_id}/courses/{course_id}/instructors")]
pub async fn assign_course_instructor(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32)>,
    body: web::Json<AssignCourseInstructorForm>,
) -> impl Responder {
    let (org_id, course_id) = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    if let Err(response) = require_course_in_organisation(db.get_ref(), course_id, org_id).await {
        return response;
    }

    let body = body.into_inner();
    if let Err(response) =
        require_instructor_in_organisation(db.get_ref(), body.instructor_id, org_id).await
    {
        return response;
    }

    let assigned_by = match session_user_id(&session) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };

    let already_assigned =
        match course_instructors::Entity::find_by_id((course_id, body.instructor_id))
            .one(db.get_ref())
            .await
        {
            Ok(assignment) => assignment.is_some(),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error checking assignment: {}", err));
            }
        };

    if already_assigned {
        return HttpResponse::Ok().json(serde_json::json!({
            "message": "Instructor is already assigned to this course"
        }));
    }

    let assignment = course_instructors::ActiveModel {
        course_id: Set(course_id),
        instructor_id: Set(body.instructor_id),
        assigned_by: Set(Some(assigned_by)),
        ..Default::default()
    };

    match assignment.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Instructor assigned to course"
        })),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to assign instructor: {}", err)),
    }
}

/// DELETE /api/organisations/{org_id}/courses/{course_id}/instructors/{instructor_id}
#[delete("/organisations/{org_id}/courses/{course_id}/instructors/{instructor_id}")]
pub async fn remove_course_instructor(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32, i32)>,
) -> impl Responder {
    let (org_id, course_id, instructor_id) = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    if let Err(response) = require_course_in_organisation(db.get_ref(), course_id, org_id).await {
        return response;
    }

    match course_instructors::Entity::delete_by_id((course_id, instructor_id))
        .exec(db.get_ref())
        .await
    {
        Ok(result) if result.rows_affected > 0 => HttpResponse::Ok().json(serde_json::json!({
            "message": "Instructor removed from course"
        })),
        Ok(_) => HttpResponse::NotFound().body("Course instructor assignment not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to remove instructor: {}", err)),
    }
}

/// POST /api/organisations/{org_id}/enroll
///
/// Body: { "user_ids": [1, 2, 3], "role": "Student" | "Instructor" }
///
/// For each user_id:
///   1. Sets users.org_id = org_id
///   2. Assigns the requested role (idempotent – skips if already assigned)
#[post("/organisations/{org_id}/enroll")]
pub async fn mass_enroll(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<MassEnrollForm>,
) -> impl Responder {
    let org_id = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    // Resolve the target role
    let target_role_name = match body.role.as_str() {
        "Instructor" => roles::RoleName::Instructor,
        "Student" => roles::RoleName::Student,
        other => {
            return HttpResponse::BadRequest().body(format!(
                "Invalid role '{}'. Use 'Instructor' or 'Student'.",
                other
            ));
        }
    };

    let role_row = match roles::Entity::find()
        .filter(roles::Column::RoleName.eq(target_role_name.clone()))
        .one(db.get_ref())
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::InternalServerError().body("Role not found in database"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error looking up role: {}", err));
        }
    };

    let txn = match db.get_ref().begin().await {
        Ok(t) => t,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err));
        }
    };

    let mut enrolled: Vec<i32> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut created_accounts: Vec<TempPasswordAccount> = Vec::new();
    let mut seen_new_emails: HashSet<String> = HashSet::new();
    let lms_admins = match lms_admin_user_ids(&txn).await {
        Ok(user_ids) => user_ids,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to check LMS admin users: {}", err));
        }
    };

    for &uid in &body.user_ids {
        // 1. Fetch user
        let user = match users::Entity::find_by_id(uid).one(&txn).await {
            Ok(Some(u)) => u,
            Ok(None) => {
                errors.push(format!("User {} not found", uid));
                continue;
            }
            Err(err) => {
                errors.push(format!("DB error for user {}: {}", uid, err));
                continue;
            }
        };

        match attach_existing_user_to_organisation(
            &txn,
            user,
            org_id,
            role_row.role_id,
            &lms_admins,
        )
        .await
        {
            Ok(user_id) => enrolled.push(user_id),
            Err(message) => errors.push(message),
        }
    }

    for new_user in &body.new_users {
        let email = new_user.email.trim().to_lowercase();

        if email.is_empty() {
            errors.push("Skipped a new user row with a blank email".to_string());
            continue;
        }

        if !validate_email(&email) {
            errors.push(format!("{} is not a valid email address", email));
            continue;
        }

        if !seen_new_emails.insert(email.clone()) {
            errors.push(format!("{} appears more than once in the import", email));
            continue;
        }

        match users::Entity::find()
            .filter(users::Column::Email.eq(email.clone()))
            .one(&txn)
            .await
        {
            Ok(Some(existing_user)) => {
                match attach_existing_user_to_organisation(
                    &txn,
                    existing_user,
                    org_id,
                    role_row.role_id,
                    &lms_admins,
                )
                .await
                {
                    Ok(user_id) => enrolled.push(user_id),
                    Err(message) => errors.push(format!("{}: {}", email, message)),
                }
            }
            Ok(None) => {
                match create_temp_password_org_user(
                    &txn,
                    org_id,
                    email.clone(),
                    new_user.first_name.clone(),
                    new_user.last_name.clone(),
                    target_role_name.clone(),
                )
                .await
                {
                    Ok(account) => {
                        enrolled.push(account.user.user_id);
                        created_accounts.push(account);
                    }
                    Err(message) => errors.push(format!("{}: {}", email, message)),
                }
            }
            Err(err) => errors.push(format!("Database error checking {}: {}", email, err)),
        }
    }

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to commit transaction: {}", err));
    }

    for account in &created_accounts {
        if let Err(err) = send_temp_password_account_email(
            &account.email,
            &account.temp_password,
            &account.role_name,
        ) {
            errors.push(format!(
                "{} account created, but the invite email could not be sent: {}",
                account.email, err
            ));
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "enrolled": enrolled,
        "created": created_accounts
            .iter()
            .map(|account| serde_json::json!({
                "user_id": account.user.user_id,
                "email": account.email,
            }))
            .collect::<Vec<serde_json::Value>>(),
        "errors": errors,
        "message": format!("{} user(s) enrolled successfully", enrolled.len())
    }))
}

/// DELETE /api/organisations/{org_id}/members/{user_id}
/// Removes a user from the organisation, its classes, and synced class enrollments.
#[delete("/organisations/{org_id}/members/{user_id}")]
pub async fn remove_org_member(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32)>,
) -> impl Responder {
    let (org_id, user_id) = path.into_inner();
    if let Err(response) =
        require_matching_session_organisation(db.get_ref(), &session, org_id).await
    {
        return response;
    }

    let user = match users::Entity::find_by_id(user_id).one(db.get_ref()).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    if user.org_id != Some(org_id) {
        return HttpResponse::Forbidden().body("Member does not belong to your organisation");
    }

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err));
        }
    };

    let class_rows = match org_classes::Entity::find()
        .filter(org_classes::Column::OrgId.eq(org_id))
        .all(&txn)
        .await
    {
        Ok(classes) => classes,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!(
                "Database error finding organisation classes: {}",
                err
            ));
        }
    };

    let class_ids = class_rows
        .iter()
        .map(|class| class.class_id)
        .collect::<Vec<i32>>();

    let class_course_rows = if class_ids.is_empty() {
        Vec::new()
    } else {
        match org_class_courses::Entity::find()
            .filter(org_class_courses::Column::ClassId.is_in(class_ids.clone()))
            .all(&txn)
            .await
        {
            Ok(rows) => rows,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding class courses: {}", err));
            }
        }
    };

    let mut course_ids_by_class: HashMap<i32, Vec<i32>> = HashMap::new();
    for class_course in class_course_rows {
        course_ids_by_class
            .entry(class_course.class_id)
            .or_default()
            .push(class_course.course_id);
    }

    let member_rows = if class_ids.is_empty() {
        Vec::new()
    } else {
        match org_class_members::Entity::find()
            .filter(org_class_members::Column::ClassId.is_in(class_ids.clone()))
            .filter(org_class_members::Column::UserId.eq(user_id))
            .all(&txn)
            .await
        {
            Ok(members) => members,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding class memberships: {}", err));
            }
        }
    };

    if !member_rows.is_empty() {
        if let Err(err) = org_class_members::Entity::delete_many()
            .filter(org_class_members::Column::ClassId.is_in(class_ids))
            .filter(org_class_members::Column::UserId.eq(user_id))
            .exec(&txn)
            .await
        {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to remove class memberships: {}", err));
        }

        for member in member_rows {
            if let Some(course_ids) = course_ids_by_class.get(&member.class_id) {
                for course_id in course_ids {
                    if let Err(err) = remove_course_enrollment_if_unassigned(
                        &txn,
                        user_id,
                        member.class_id,
                        *course_id,
                    )
                    .await
                    {
                        return HttpResponse::InternalServerError().body(format!(
                            "Failed to sync enrollment for user {}: {}",
                            user_id, err
                        ));
                    }
                }
            }
        }
    }

    let mut active_user = sea_orm::IntoActiveModel::into_active_model(user);
    active_user.org_id = Set(None);

    match active_user.update(&txn).await {
        Ok(_) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to remove member: {}", err));
        }
    }

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to commit member removal: {}", err));
    }

    HttpResponse::Ok().body("Member removed from organisation")
}

/// GET /api/users/all  –  users visible to this organisation admin for CSV/Excel matching
#[get("/users/all")]
pub async fn list_all_users(db: web::Data<DatabaseConnection>, session: Session) -> impl Responder {
    if let Err(response) = require_session_organisation(db.get_ref(), &session).await {
        return response;
    }

    let lms_admins = match lms_admin_user_ids(db.get_ref()).await {
        Ok(user_ids) => user_ids,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to check LMS admin users: {}", err));
        }
    };

    match users::Entity::find().all(db.get_ref()).await {
        Ok(users) => {
            let safe: Vec<serde_json::Value> = users
                .iter()
                .filter(|u| !lms_admins.contains(&u.user_id))
                .map(|u| {
                    serde_json::json!({
                        "user_id": u.user_id,
                        "first_name": u.first_name,
                        "last_name": u.last_name,
                        "email": u.email,
                        "org_id": u.org_id,
                    })
                })
                .collect();
            HttpResponse::Ok().json(safe)
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

/// GET /api/users/unassigned  –  users not yet in any organisation (for the picker)
#[get("/users/unassigned")]
pub async fn list_unassigned_users(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    if let Err(response) = require_session_organisation(db.get_ref(), &session).await {
        return response;
    }

    let lms_admins = match lms_admin_user_ids(db.get_ref()).await {
        Ok(user_ids) => user_ids,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to check LMS admin users: {}", err));
        }
    };

    match users::Entity::find()
        .filter(users::Column::OrgId.is_null())
        .all(db.get_ref())
        .await
    {
        Ok(users) => {
            let safe: Vec<serde_json::Value> = users
                .iter()
                .filter(|u| !lms_admins.contains(&u.user_id))
                .map(|u| {
                    serde_json::json!({
                        "user_id": u.user_id,
                        "first_name": u.first_name,
                        "last_name": u.last_name,
                        "email": u.email,
                    })
                })
                .collect();
            HttpResponse::Ok().json(safe)
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}
