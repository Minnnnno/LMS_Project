use actix_session::Session;
use actix_web::{
    HttpRequest, HttpResponse, Responder, delete, get,
    http::{StatusCode, header},
    post, web,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr,
    EntityTrait, QueryFilter, Set, TransactionTrait,
};
use std::collections::{HashMap, HashSet};
use tera::{Context, Tera};
use validator::{validate_email, Validate};

use crate::entity::{
    course_instructors, courses, organisation_signup_requests, organisations, roles, user_roles,
    users,
};
use crate::models::organisation::{
    AssignCourseInstructorForm, CourseInstructorCourseDto, CourseInstructorDto,
    CourseInstructorSummaryDto, CreateOrganisationForm, InviteInstructorForm, MassEnrollForm,
    OrgMemberDto, OrganisationSignupForm,
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

    let (
        requester_user_id,
        admin_first_name,
        admin_last_name,
        admin_email,
        admin_password_hash,
    ) = if let Some(user) = &current_user {
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
            Some(form
                .admin_first_name
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string()),
            Some(form
                .admin_last_name
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string()),
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
                Some("An organisation signup request for this admin email is already pending approval."),
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

    let inserted_user = new_user
        .insert(db)
        .await
        .map_err(|err| format!("Failed to create {} account: {}", role_label(&role_name), err))?;

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

async fn assign_role_id_if_missing<C>(
    db: &C,
    user_id: i32,
    role_id: i32,
) -> Result<(), DbErr>
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
        return Err(format!("User {} is an LMS admin and cannot be added", user_id));
    }

    if let Some(existing_org_id) = user.org_id {
        if existing_org_id == org_id {
            return Err(format!("User {} already belongs to this organisation", user_id));
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
/// Removes a user from the organisation (sets org_id to NULL)
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

    let mut active_user = sea_orm::IntoActiveModel::into_active_model(user);
    active_user.org_id = Set(None);

    match active_user.update(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("Member removed from organisation"),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to remove member: {}", err))
        }
    }
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

    match users::Entity::find().all(db.get_ref()).await
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
