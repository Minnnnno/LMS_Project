use actix_session::Session;
use actix_web::{
    HttpResponse, Responder, delete, get,
    http::{StatusCode, header},
    post, web,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, DbErr,
    EntityTrait, QueryFilter, Set, TransactionTrait,
};
use tera::{Context, Tera};
use validator::Validate;

use crate::entity::{organisations, roles, user_roles, users};
use crate::models::organisation::{
    CreateOrganisationForm, InviteInstructorForm, MassEnrollForm, OrgMemberDto,
    OrganisationSignupForm,
};
use crate::services::course_service::{get_session_user_org_id, has_role};
use crate::services::email_verification_service::{
    create_email_verification_token, send_verification_email,
};
use crate::services::mailer_service::{send_mail_message, MailRequest};
use crate::services::organisation_service;
use crate::services::user_service::{assign_role_to_user, hash_password, sign_user_into_session};
use crate::ssr::pages::{build_page_context, render_page};
use uuid::Uuid;

const ORG_DASHBOARD_PATH: &str = "/organisation";

// ── Session helpers ────────────────────────────────────────────────────────────

// ── Page route ─────────────────────────────────────────────────────────────────

#[get("/organisation")]
pub async fn organisation_page(session: Session) -> impl Responder {
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

    let org_name = form.org_name.trim().to_string();
    let org_slug = form.org_slug.trim().to_lowercase();
    let org_type = optional_trimmed(form.org_type.as_deref());
    let website_url = optional_trimmed(form.website_url.as_deref());

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            println!("Organisation signup transaction error: {:?}", err);
            return render_signup_page(
                &session,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some("Unable to create the organisation right now."),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }
    };

    match organisations::Entity::find()
        .filter(organisations::Column::OrgSlug.eq(org_slug.clone()))
        .one(&txn)
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

    let new_org = organisations::ActiveModel {
        org_name: Set(org_name),
        org_slug: Set(Some(org_slug)),
        org_type: Set(org_type),
        website_url: Set(website_url),
        ..Default::default()
    };

    let inserted_org = match new_org.insert(&txn).await {
        Ok(org) => org,
        Err(err) => {
            println!("Organisation insert error: {:?}", err);
            return render_signup_page(
                &session,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some("Unable to create the organisation right now."),
                current_user.as_ref(),
                None,
                Some(&form),
            );
        }
    };

    let mut new_admin_verification: Option<(String, String)> = None;

    let signed_in_user = if let Some(user) = current_user {
        let refreshed_user = match users::Entity::find_by_id(user.user_id).one(&txn).await {
            Ok(Some(refreshed_user)) => refreshed_user,
            Ok(None) => {
                return render_signup_page(
                    &session,
                    StatusCode::BAD_REQUEST,
                    Some("Your account could not be found. Please sign in again."),
                    Some(&user),
                    None,
                    Some(&form),
                );
            }
            Err(err) => {
                println!("Current user lookup error: {:?}", err);
                return render_signup_page(
                    &session,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some("Unable to verify your account right now."),
                    Some(&user),
                    None,
                    Some(&form),
                );
            }
        };

        if refreshed_user.org_id.is_some() {
            return render_signup_page(
                &session,
                StatusCode::BAD_REQUEST,
                Some("User already belongs to an organisation."),
                Some(&refreshed_user),
                None,
                Some(&form),
            );
        }

        let mut active_user = sea_orm::IntoActiveModel::into_active_model(refreshed_user);
        active_user.org_id = Set(Some(inserted_org.org_id));
        let updated_user = match active_user.update(&txn).await {
            Ok(user) => user,
            Err(err) => {
                println!("Current user organisation update error: {:?}", err);
                return render_signup_page(
                    &session,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some("Unable to attach your account to the organisation."),
                    Some(&user),
                    None,
                    Some(&form),
                );
            }
        };

        if let Err(err) = assign_role_if_missing(
            &txn,
            updated_user.user_id,
            roles::RoleName::OrganisationAdmin,
        )
        .await
        {
            println!("Organisation admin role assignment error: {:?}", err);
            return render_signup_page(
                &session,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some("Unable to assign the organisation admin role."),
                Some(&updated_user),
                None,
                Some(&form),
            );
        }

        updated_user
    } else {
        let admin_email = form
            .admin_email
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_lowercase();

        match users::Entity::find()
            .filter(users::Column::Email.eq(admin_email.clone()))
            .one(&txn)
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

        let new_user = users::ActiveModel {
            first_name: Set(form
                .admin_first_name
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string()),
            last_name: Set(form
                .admin_last_name
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string()),
            email: Set(admin_email),
            password_hash: Set(Some(password_hash)),
            auth_provider: Set("password".to_string()),
            org_id: Set(Some(inserted_org.org_id)),
            email_verified: Set(false),
            must_change_password: Set(false),
            ..Default::default()
        };

        let inserted_user = match new_user.insert(&txn).await {
            Ok(user) => user,
            Err(err) => {
                println!("Admin user insert error: {:?}", err);
                return render_signup_page(
                    &session,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some("Unable to create the admin account right now."),
                    None,
                    None,
                    Some(&form),
                );
            }
        };

        if let Err(err) = assign_role_to_user(
            &txn,
            inserted_user.user_id,
            roles::RoleName::OrganisationAdmin,
        )
        .await
        {
            println!("Organisation admin role assignment error: {:?}", err);
            return render_signup_page(
                &session,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some("Unable to assign the organisation admin role."),
                Some(&inserted_user),
                None,
                Some(&form),
            );
        }

        let verification_token =
            match create_email_verification_token(&txn, inserted_user.user_id).await {
                Ok(token) => token,
                Err(err) => {
                    println!("Organisation admin verification token error: {:?}", err);
                    return render_signup_page(
                        &session,
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Some("Unable to prepare email verification right now."),
                        Some(&inserted_user),
                        None,
                        Some(&form),
                    );
                }
            };

        new_admin_verification = Some((inserted_user.email.clone(), verification_token));

        inserted_user
    };

    if let Err(err) = txn.commit().await {
        println!("Organisation signup commit error: {:?}", err);
        return render_signup_page(
            &session,
            StatusCode::INTERNAL_SERVER_ERROR,
            Some("Unable to finish creating the organisation right now."),
            Some(&signed_in_user),
            None,
            Some(&form),
        );
    }

    if creates_new_admin_account {
        if let Some((email, token)) = new_admin_verification {
            if let Err(err) = send_verification_email(&email, &token) {
                println!("Organisation admin verification email error: {}", err);
                return render_signup_page(
                    &session,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some("Organisation created, but the verification email could not be sent."),
                    Some(&signed_in_user),
                    None,
                    Some(&form),
                );
            }
        }

        let _ = session.insert(
            "flash_success",
            "Organisation created. Please check your email to verify the admin account before signing in.",
        );
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/login"))
            .finish();
    }

    if let Err(message) = sign_user_into_session(db.get_ref(), &session, &signed_in_user).await {
        println!("Organisation signup session error: {}", message);
        return render_signup_page(
            &session,
            StatusCode::INTERNAL_SERVER_ERROR,
            Some("Organisation created, but we could not sign you in automatically."),
            Some(&signed_in_user),
            None,
            Some(&form),
        );
    }

    HttpResponse::Found()
        .insert_header((header::LOCATION, ORG_DASHBOARD_PATH))
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

    if let Some(error) = error {
        context.insert("error", error);
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

fn send_instructor_invite_email(email: &str, temp_password: &str) -> Result<(), String> {
    let body = format!(
        "You have been invited as an instructor on SkillUp LMS.\n\nLogin email: {}\nTemporary password: {}\n\nSign in here: {}\n\nYou will be asked to change this temporary password after signing in.",
        email,
        temp_password,
        login_url()
    );

    send_mail_message(MailRequest {
        to: email.to_string(),
        subject: "Your SkillUp LMS instructor account".to_string(),
        body,
        is_html: false,
    })
}

async fn assign_role_if_missing<C>(
    db: &C,
    user_id: i32,
    role_name: roles::RoleName,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let role = roles::Entity::find()
        .filter(roles::Column::RoleName.eq(role_name))
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
    if !has_role(&session, "LMS Admin") {
        return HttpResponse::Forbidden().body("LMS Admin role required");
    }

    let org_id = path.into_inner();
    match organisations::Entity::delete_by_id(org_id)
        .exec(db.get_ref())
        .await
    {
        Ok(res) if res.rows_affected > 0 => HttpResponse::Ok().body("Organisation deleted"),
        Ok(_) => HttpResponse::NotFound().body("Organisation not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to delete organisation: {}", err)),
    }
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

    let temp_password = generate_temp_password();
    let password_hash = match hash_password(temp_password.clone()).await {
        Ok(hash) => hash,
        Err(message) => return HttpResponse::InternalServerError().body(message),
    };
    let (first_name, last_name) = name_from_email(&email);

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err));
        }
    };

    let new_user = users::ActiveModel {
        first_name: Set(first_name),
        last_name: Set(last_name),
        email: Set(email.clone()),
        password_hash: Set(Some(password_hash)),
        auth_provider: Set("password".to_string()),
        org_id: Set(Some(org_id)),
        email_verified: Set(true),
        must_change_password: Set(true),
        ..Default::default()
    };

    let inserted_user = match new_user.insert(&txn).await {
        Ok(user) => user,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to create instructor account: {}", err));
        }
    };

    if let Err(err) =
        assign_role_if_missing(&txn, inserted_user.user_id, roles::RoleName::Instructor).await
    {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to assign instructor role: {}", err));
    }

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to commit instructor invite: {}", err));
    }

    match send_instructor_invite_email(&email, &temp_password) {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "user_id": inserted_user.user_id,
            "email": email,
            "message": "Instructor invited successfully"
        })),
        Err(err) => HttpResponse::InternalServerError().body(format!(
            "Instructor account created, but the invite email could not be sent: {}",
            err
        )),
    }
}

// ── Mass enrollment ────────────────────────────────────────────────────────────

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
        .filter(roles::Column::RoleName.eq(target_role_name))
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

        if let Some(existing_org_id) = user.org_id {
            if existing_org_id != org_id {
                errors.push(format!(
                    "User {} already belongs to another organisation",
                    uid
                ));
                continue;
            }
        }

        // 2. Set org_id on the user
        let mut active_user = sea_orm::IntoActiveModel::into_active_model(user);
        active_user.org_id = Set(Some(org_id));
        if let Err(err) = active_user.update(&txn).await {
            errors.push(format!("Failed to set org for user {}: {}", uid, err));
            continue;
        }

        // 3. Assign role (idempotent)
        let already_has_role = match user_roles::Entity::find()
            .filter(user_roles::Column::UserId.eq(uid))
            .filter(user_roles::Column::RoleId.eq(role_row.role_id))
            .one(&txn)
            .await
        {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(err) => {
                errors.push(format!("Role check error for user {}: {}", uid, err));
                continue;
            }
        };

        if !already_has_role {
            let new_ur = user_roles::ActiveModel {
                user_id: Set(uid),
                role_id: Set(role_row.role_id),
            };
            if let Err(err) = new_ur.insert(&txn).await {
                errors.push(format!("Failed to assign role for user {}: {}", uid, err));
                continue;
            }
        }

        enrolled.push(uid);
    }

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to commit transaction: {}", err));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "enrolled": enrolled,
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
    let org_id = match require_session_organisation(db.get_ref(), &session).await {
        Ok(org_id) => org_id,
        Err(response) => return response,
    };

    match users::Entity::find()
        .filter(
            Condition::any()
                .add(users::Column::OrgId.is_null())
                .add(users::Column::OrgId.eq(org_id)),
        )
        .all(db.get_ref())
        .await
    {
        Ok(users) => {
            let safe: Vec<serde_json::Value> = users
                .iter()
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

/// GET /api/users/unassigned  –  users not yet in any organisation (for the picker)
#[get("/users/unassigned")]
pub async fn list_unassigned_users(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    if let Err(response) = require_session_organisation(db.get_ref(), &session).await {
        return response;
    }

    match users::Entity::find()
        .filter(users::Column::OrgId.is_null())
        .all(db.get_ref())
        .await
    {
        Ok(users) => {
            let safe: Vec<serde_json::Value> = users
                .iter()
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
