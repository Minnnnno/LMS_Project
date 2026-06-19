use crate::entity::{roles, user_roles, users};
use crate::models::student::ChangePasswordForm;
use crate::models::user::{ForgotPasswordForm, LoginForm, RegisterForm, ResetPasswordForm};
use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    QueryFilter, Set, TransactionTrait,
};
use serde::Deserialize;

use crate::ssr::pages::{build_page_context, render_page};
use crate::services::email_verification_service::{
    create_email_verification_token, send_verification_email, verify_email_token, VerifyEmailError,
};
use crate::services::password_reset_service::{
    create_password_reset_token, reset_password_with_token, send_reset_email, ResetPasswordError,
};
use crate::services::remember_me_service::{
    create_remember_me_cookie, forget_remember_me_cookie, revoke_remember_me_token,
    REMEMBER_ME_COOKIE,
};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordVerifier},
};
use crate::services::user_service::{
    assign_role_to_user,
    google_client_id,
    google_client_secret,
    google_redirect_uri,
    hash_password,
    is_logged_in,
    load_user_roles,
    redirect_home,
    sign_user_into_session,
    store_roles_in_session,
};
use tera::{Context, Tera};
use uuid::Uuid;
use validator::Validate;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

#[derive(Debug, Deserialize)]
pub struct GoogleAuthQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    email: String,
    email_verified: bool,
    given_name: Option<String>,
    family_name: Option<String>,
    name: Option<String>,
}

fn redirect_login_with_error(session: &Session, error_message: &str) -> HttpResponse {
    if let Err(err) = session.insert("flash_error", error_message) {
        println!("Session flash insert error: {:?}", err);
    }

    HttpResponse::Found()
        .insert_header((header::LOCATION, "/login"))
        .finish()
}

fn render_register_error(
    error_message: &str,
    first_name: &str,
    last_name: &str,
    email: &str,
) -> HttpResponse {
    let tera = Tera::new("../frontend/templates/**/*").expect("Failed to load templates");

    let mut context = Context::new();
    context.insert("is_logged_in", &false);
    context.insert("role_names", &Vec::<String>::new());
    context.insert("error", error_message);
    context.insert("first_name", first_name);
    context.insert("last_name", last_name);
    context.insert("email", email);

    let html = tera
        .render("register.html", &context)
        .expect("Failed to render register.html");

    HttpResponse::BadRequest()
        .content_type("text/html")
        .body(html)
}

fn render_login_error(error_message: &str, email: &str) -> HttpResponse {
    let tera = Tera::new("../frontend/templates/**/*").expect("Failed to load templates");

    let mut context = Context::new();
    context.insert("is_logged_in", &false);
    context.insert("role_names", &Vec::<String>::new());
    context.insert("error", error_message);
    context.insert("email", email);

    let html = tera
        .render("login.html", &context)
        .expect("Failed to render login.html");

    HttpResponse::BadRequest()
        .content_type("text/html")
        .body(html)
}

fn request_user_agent(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}

fn request_ip_address(req: &HttpRequest) -> Option<String> {
    req.connection_info()
        .realip_remote_addr()
        .map(|value| value.to_string())
}

#[get("/login")]
pub async fn login(session: Session) -> impl Responder {
    if is_logged_in(&session) {
        if session
            .get::<bool>("must_change_password")
            .ok()
            .flatten()
            .unwrap_or(false)
        {
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/change-password"))
                .finish();
        }

        return redirect_home();
    }

    let tera = Tera::new("../frontend/templates/**/*").expect("Failed to load templates");

    let mut context = build_page_context(&session);
    if let Ok(Some(success)) = session.get::<String>("flash_success") {
        context.insert("success", &success);
        session.remove("flash_success");
    }
    if let Ok(Some(error)) = session.get::<String>("flash_error") {
        context.insert("error", &error);
        session.remove("flash_error");
    }

    let html = tera
        .render("login.html", &context)
        .expect("Failed to render login.html");

    HttpResponse::Ok().content_type("text/html").body(html)
}

fn render_change_password_page(
    session: &Session,
    status: actix_web::http::StatusCode,
    error: Option<&str>,
) -> HttpResponse {
    let tera = Tera::new("../frontend/templates/**/*").expect("Failed to load templates");
    let mut context = build_page_context(session);
    if let Some(error) = error {
        context.insert("error", error);
    }

    let html = tera
        .render("change_password.html", &context)
        .expect("Failed to render change_password.html");

    HttpResponse::build(status)
        .content_type("text/html")
        .body(html)
}

#[get("/change-password")]
pub async fn change_password_page(session: Session) -> impl Responder {
    if !is_logged_in(&session) {
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/login"))
            .finish();
    }

    if !session
        .get::<bool>("must_change_password")
        .ok()
        .flatten()
        .unwrap_or(false)
    {
        return redirect_home();
    }

    render_change_password_page(&session, actix_web::http::StatusCode::OK, None)
}

#[post("/change-password")]
pub async fn change_password_submit(
    db: web::Data<DatabaseConnection>,
    session: Session,
    form: web::Form<ChangePasswordForm>,
) -> impl Responder {
    let user_id = match session.get::<i32>("user_id").ok().flatten() {
        Some(user_id) => user_id,
        None => {
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
    };

    let form = form.into_inner();
    if let Err(errors) = form.validate() {
        println!("Forced password change validation error: {:?}", errors);
        return render_change_password_page(
            &session,
            actix_web::http::StatusCode::BAD_REQUEST,
            Some("Please check your password details."),
        );
    }

    if form.new_password != form.confirm_password {
        return render_change_password_page(
            &session,
            actix_web::http::StatusCode::BAD_REQUEST,
            Some("New password and confirm password do not match."),
        );
    }

    let user = match users::Entity::find_by_id(user_id).one(db.get_ref()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            session.purge();
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
        Err(err) => {
            println!("Forced password change user lookup error: {:?}", err);
            return render_change_password_page(
                &session,
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                Some("Unable to load your account right now."),
            );
        }
    };

    let password_hash = match &user.password_hash {
        Some(password_hash) => password_hash,
        None => {
            return render_change_password_page(
                &session,
                actix_web::http::StatusCode::BAD_REQUEST,
                Some("This account does not have a password to change."),
            );
        }
    };

    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(err) => {
            println!("Forced password hash parse error: {:?}", err);
            return render_change_password_page(
                &session,
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                Some("Unable to verify your current password."),
            );
        }
    };

    if Argon2::default()
        .verify_password(form.current_password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return render_change_password_page(
            &session,
            actix_web::http::StatusCode::UNAUTHORIZED,
            Some("Current password is incorrect."),
        );
    }

    let new_password_hash = match hash_password(form.new_password).await {
        Ok(hash) => hash,
        Err(message) => {
            return render_change_password_page(
                &session,
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                Some(&message),
            );
        }
    };

    let mut active_user = user.into_active_model();
    active_user.password_hash = Set(Some(new_password_hash));
    active_user.auth_provider = Set("password".to_string());
    active_user.must_change_password = Set(false);

    match active_user.update(db.get_ref()).await {
        Ok(_) => {
            let _ = session.insert("must_change_password", false);
            HttpResponse::Found()
                .insert_header((header::LOCATION, "/"))
                .finish()
        }
        Err(err) => {
            println!("Forced password change update error: {:?}", err);
            render_change_password_page(
                &session,
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                Some("Unable to update your password right now."),
            )
        }
    }
}

#[get("/register")]
pub async fn register(session: Session) -> impl Responder {
    if is_logged_in(&session) {
        return redirect_home();
    }

    render_page("register.html", &session)
}

#[post("/register")]
pub async fn register_submit(
    db: web::Data<DatabaseConnection>,
    session: Session,
    form: web::Form<RegisterForm>,
) -> impl Responder {
    let form: RegisterForm = form.into_inner();

    if let Err(errors) = form.validate() {
        println!("{:?}", errors);

        let field_errors = errors.field_errors();

        let error_message = if field_errors.contains_key("first_name") {
            "First name is required."
        } else if field_errors.contains_key("last_name") {
            "Last name is required."
        } else if field_errors.contains_key("email") {
            "Please enter a valid email address."
        } else if field_errors.contains_key("password") {
            "Password must be between 8 and 128 characters."
        } else if field_errors.contains_key("confirm_password") {
            "Confirm password must be between 8 and 128 characters."
        } else {
            "Please check your registration details."
        };

        return render_register_error(
            error_message,
            &form.first_name,
            &form.last_name,
            &form.email,
        );
    }

    if form.password != form.confirm_password {
        return render_register_error(
            "Passwords do not match.",
            &form.first_name,
            &form.last_name,
            &form.email,
        );
    }

    let password_hash = match hash_password(form.password.clone()).await {
        Ok(hash) => hash,
        Err(message) => {
            return render_register_error(&message, &form.first_name, &form.last_name, &form.email);
        }
    };
    // trim and sanitize input
    let first_name = form.first_name.trim().to_string();
    let last_name = form.last_name.trim().to_string();
    let email = form.email.trim().to_lowercase();

    // Check if the email already exists in the database
    if let Ok(Some(_)) = users::Entity::find()
        .filter(users::Column::Email.eq(email.clone()))
        .one(db.get_ref())
        .await
    {
        return render_register_error(
            "Email is already registered.",
            &first_name,
            &last_name,
            &email,
        );
    }

    println!("First name: {}", first_name);
    println!("Last name: {}", last_name);
    println!("Email: {}", email);
    println!("Password hash: {}", password_hash);

    let new_user = users::ActiveModel {
        first_name: Set(first_name.clone()),
        last_name: Set(last_name.clone()),
        email: Set(email.clone()),
        password_hash: Set(Some(password_hash)),
        auth_provider: Set("password".to_string()),
        email_verified: Set(false),
        must_change_password: Set(false),

        // org_id is not set here.
        // This lets the database use its default value.
        ..Default::default()
    };

    let txn = match db.get_ref().begin().await {
        Ok(txn) => txn,
        Err(err) => {
            println!("Registration transaction error: {:?}", err);
            return render_register_error(
                "Unable to register your account right now.",
                &first_name,
                &last_name,
                &email,
            );
        }
    };

    let inserted_user = match new_user.insert(&txn).await {
        Ok(user) => user,
        Err(err) => {
            println!("Insert user error: {:?}", err);
            return render_register_error(
                "This email is already in use. Please log in or use a different email.",
                &first_name,
                &last_name,
                &email,
            );
        }
    };

    if let Err(err) =
        assign_role_to_user(&txn, inserted_user.user_id, roles::RoleName::Student).await
    {
        println!("Assign student role error: {:?}", err);
        return render_register_error(
            "Unable to assign the student role right now.",
            &first_name,
            &last_name,
            &email,
        );
    }

    let verification_token = match create_email_verification_token(&txn, inserted_user.user_id).await
    {
        Ok(token) => token,
        Err(err) => {
            println!("Email verification token error: {:?}", err);
            return render_register_error(
                "Unable to prepare email verification right now.",
                &first_name,
                &last_name,
                &email,
            );
        }
    };

    if let Err(err) = txn.commit().await {
        println!("Registration commit error: {:?}", err);
        return render_register_error(
            "Unable to register your account right now.",
            &first_name,
            &last_name,
            &email,
        );
    }

    let flash_message = match send_verification_email(&inserted_user.email, &verification_token) {
        Ok(_) => "User registered successfully. Please check your email to verify your account.",
        Err(err) => {
            println!("Verification email send error: {}", err);
            "User registered successfully, but the verification email could not be sent. Please request a new verification email after logging in."
        }
    };

    if let Err(err) = session.insert("flash_success", flash_message) {
        println!("Session flash insert error: {:?}", err);
    }

    HttpResponse::Found()
        .insert_header((header::LOCATION, "/login"))
        .finish()
}

#[post("/login")]
pub async fn login_submit(
    db: web::Data<DatabaseConnection>,
    req: HttpRequest,
    session: Session,
    form: web::Form<LoginForm>,
) -> impl Responder {
    let form: LoginForm = form.into_inner();

    if let Err(errors) = form.validate() {
        println!("{:?}", errors);
        return render_login_error("Please enter a valid email and password.", &form.email);
    }

    let email = form.email.trim().to_lowercase();
    let password = form.password.clone();

    let user = match users::Entity::find()
        .filter(users::Column::Email.eq(email.clone()))
        .one(db.get_ref())
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return render_login_error("Incorrect email or password.", &email);
        }
        Err(err) => {
            println!("Login lookup error: {:?}", err);
            return render_login_error("Unable to process login at this time.", &email);
        }
    };

    let password_hash = match &user.password_hash {
        Some(password_hash) => password_hash,
        None => {
            return render_login_error("Please sign in with Google for this account.", &email);
        }
    };

    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(err) => {
            println!("Password hash parse error: {:?}", err);
            return render_login_error("Incorrect email or password.", &email);
        }
    };

    if Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return render_login_error("Incorrect email or password.", &email);
    }

    let (role_ids, role_names) = match load_user_roles(db.get_ref(), user.user_id).await {
        Ok(user_roles) => user_roles,
        Err(err) => {
            println!("User role lookup error: {:?}", err);
            return render_login_error("Unable to process login at this time.", &email);
        }
    };

    session.renew();

    if let Err(err) = session.insert("user_id", user.user_id) {
        println!("Session insert error: {:?}", err);
    }
    if let Err(err) = session.insert("user_email", user.email.clone()) {
        println!("Session insert error: {:?}", err);
    }
    if let Err(err) = session.insert("email_verified", user.email_verified) {
        println!("Session insert error: {:?}", err);
    }
    if let Err(err) = session.insert("must_change_password", user.must_change_password) {
        println!("Session insert error: {:?}", err);
    }

    let is_lms_admin = role_names.iter().any(|role| role == "LMS Admin");
    let must_change_password = user.must_change_password;

    store_roles_in_session(&session, role_ids, role_names);

    println!("Login successful. Stored user_id: {}", user.user_id);

    let remember_cookie = if form.remember_me.is_some() && !must_change_password {
        match create_remember_me_cookie(
            db.get_ref(),
            user.user_id,
            request_user_agent(&req),
            request_ip_address(&req),
        )
        .await
        {
            Ok(cookie) => Some(cookie),
            Err(err) => {
                println!("Remember-me token create error: {:?}", err);
                None
            }
        }
    } else {
        None
    };

    let mut response = if must_change_password {
        HttpResponse::Found()
            .insert_header((header::LOCATION, "/change-password"))
            .finish()
    } else if is_lms_admin {
        HttpResponse::Found()
            .insert_header((header::LOCATION, "/admin/dashboard"))
            .finish()
    } else {
        redirect_home()
    };

    if let Some(cookie) = remember_cookie {
        if let Err(err) = response.add_cookie(&cookie) {
            println!("Remember-me cookie add error: {:?}", err);
        }
    }

    response
}

#[get("/auth/google")]
pub async fn google_auth(session: Session) -> impl Responder {
    if is_logged_in(&session) {
        return redirect_home();
    }

    let client_id = match google_client_id() {
        Ok(client_id) => client_id,
        Err(message) => return redirect_login_with_error(&session, &message),
    };

    let redirect_uri = google_redirect_uri();
    let state = Uuid::new_v4().to_string();

    if let Err(err) = session.insert("google_oauth_state", state.clone()) {
        println!("Session state insert error: {:?}", err);
        return redirect_login_with_error(&session, "Unable to start Google login at this time.");
    }

    let mut auth_url =
        reqwest::Url::parse(GOOGLE_AUTH_URL).expect("Google auth URL should be valid");
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email profile")
        .append_pair("state", &state)
        .append_pair("prompt", "select_account");

    HttpResponse::Found()
        .insert_header((header::LOCATION, auth_url.to_string()))
        .finish()
}

#[get("/auth/google/callback")]
pub async fn google_callback(
    db: web::Data<DatabaseConnection>,
    session: Session,
    query: web::Query<GoogleAuthQuery>,
) -> impl Responder {
    if let Some(error) = &query.error {
        println!("Google OAuth error: {}", error);
        session.remove("google_oauth_state");
        return redirect_login_with_error(&session, "Google login was cancelled or denied.");
    }

    let expected_state = match session.get::<String>("google_oauth_state").ok().flatten() {
        Some(state) => state,
        None => {
            return redirect_login_with_error(
                &session,
                "Google login session expired. Please try again.",
            );
        }
    };
    session.remove("google_oauth_state");

    if query.state.as_deref() != Some(expected_state.as_str()) {
        return redirect_login_with_error(
            &session,
            "Google login could not be verified. Please try again.",
        );
    }

    let code = match &query.code {
        Some(code) => code,
        None => {
            return redirect_login_with_error(
                &session,
                "Google did not return an authorization code.",
            );
        }
    };

    let client_id = match google_client_id() {
        Ok(client_id) => client_id,
        Err(message) => return redirect_login_with_error(&session, &message),
    };
    let client_secret = match google_client_secret() {
        Ok(client_secret) => client_secret,
        Err(message) => return redirect_login_with_error(&session, &message),
    };
    let redirect_uri = google_redirect_uri();

    let http_client = reqwest::Client::new();
    let token_response = match http_client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            println!("Google token request error: {:?}", err);
            return redirect_login_with_error(
                &session,
                "Unable to connect to Google login right now.",
            );
        }
    };

    if !token_response.status().is_success() {
        let status = token_response.status();
        let body = token_response.text().await.unwrap_or_default();
        println!("Google token response error: {} {}", status, body);
        return redirect_login_with_error(
            &session,
            "Google login failed while verifying your account.",
        );
    }

    let token: GoogleTokenResponse = match token_response.json().await {
        Ok(token) => token,
        Err(err) => {
            println!("Google token parse error: {:?}", err);
            return redirect_login_with_error(
                &session,
                "Google login returned an unexpected response.",
            );
        }
    };

    let userinfo_response = match http_client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(&token.access_token)
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            println!("Google userinfo request error: {:?}", err);
            return redirect_login_with_error(
                &session,
                "Unable to load your Google profile right now.",
            );
        }
    };

    if !userinfo_response.status().is_success() {
        let status = userinfo_response.status();
        let body = userinfo_response.text().await.unwrap_or_default();
        println!("Google userinfo response error: {} {}", status, body);
        return redirect_login_with_error(
            &session,
            "Google login failed while loading your profile.",
        );
    }

    let google_user: GoogleUserInfo = match userinfo_response.json().await {
        Ok(userinfo) => userinfo,
        Err(err) => {
            println!("Google userinfo parse error: {:?}", err);
            return redirect_login_with_error(&session, "Google profile data could not be read.");
        }
    };

    if !google_user.email_verified {
        return redirect_login_with_error(
            &session,
            "Your Google email must be verified before you can sign in.",
        );
    }

    let email = google_user.email.trim().to_lowercase();
    let user = match users::Entity::find()
        .filter(users::Column::Email.eq(email.clone()))
        .one(db.get_ref())
        .await
    {
        Ok(Some(user)) => {
            if user.email_verified {
                user
            } else {
                let mut active_user = user.into_active_model();
                active_user.email_verified = Set(true);
                match active_user.update(db.get_ref()).await {
                    Ok(user) => user,
                    Err(err) => {
                        println!("Google email verification update error: {:?}", err);
                        return redirect_login_with_error(
                            &session,
                            "Unable to update your verified email status.",
                        );
                    }
                }
            }
        }
        Ok(None) => {
            let fallback_name = email
                .split('@')
                .next()
                .filter(|value| !value.is_empty())
                .unwrap_or("Google")
                .to_string();
            let first_name = google_user
                .given_name
                .or_else(|| google_user.name.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(fallback_name)
                .trim()
                .to_string();
            let last_name = google_user
                .family_name
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "User".to_string())
                .trim()
                .to_string();
            let new_user = users::ActiveModel {
                first_name: Set(first_name),
                last_name: Set(last_name),
                email: Set(email.clone()),
                password_hash: Set(None),
                auth_provider: Set("google".to_string()),
                email_verified: Set(true),
                must_change_password: Set(false),
                ..Default::default()
            };

            let txn = match db.get_ref().begin().await {
                Ok(txn) => txn,
                Err(err) => {
                    println!("Google registration transaction error: {:?}", err);
                    return redirect_login_with_error(
                        &session,
                        "Unable to create your account from Google login.",
                    );
                }
            };

            let user = match new_user.insert(&txn).await {
                Ok(user) => user,
                Err(err) => {
                    println!("Google user insert error: {:?}", err);
                    return redirect_login_with_error(
                        &session,
                        "Unable to create your account from Google login.",
                    );
                }
            };

            if let Err(err) =
                assign_role_to_user(&txn, user.user_id, roles::RoleName::Student).await
            {
                println!("Google student role assignment error: {:?}", err);
                return redirect_login_with_error(
                    &session,
                    "Unable to assign the student role to your account.",
                );
            }

            if let Err(err) = txn.commit().await {
                println!("Google registration commit error: {:?}", err);
                return redirect_login_with_error(
                    &session,
                    "Unable to create your account from Google login.",
                );
            }

            user
        }
        Err(err) => {
            println!("Google login user lookup error: {:?}", err);
            return redirect_login_with_error(
                &session,
                "Unable to process Google login at this time.",
            );
        }
    };

    if let Err(message) = sign_user_into_session(db.get_ref(), &session, &user).await {
        return redirect_login_with_error(&session, &message);
    }

    println!("Google login successful. Stored user_id: {}", user.user_id);

    if user.must_change_password {
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/change-password"))
            .finish();
    }

    redirect_home()
}

#[get("/auth/verify-email")]
pub async fn verify_email(
    db: web::Data<DatabaseConnection>,
    session: Session,
    query: web::Query<VerifyEmailQuery>,
) -> impl Responder {
    let token = query.token.trim();

    if token.is_empty() {
        return redirect_login_with_error(&session, "Verification link is missing a token.");
    }

    match verify_email_token(db.get_ref(), token).await {
        Ok(user) => {
            let is_current_user = session.get::<i32>("user_id").ok().flatten() == Some(user.user_id);
            if is_current_user {
                let _ = session.insert("email_verified", true);
            }
            let redirect_path = if is_current_user { "/profile" } else { "/login" };
            if is_current_user {
                let _ = session.insert(
                    "profile_success",
                    "Email verified successfully. You can now continue.",
                );
            } else {
                let _ = session.insert(
                    "flash_success",
                    "Email verified successfully. You can now continue.",
                );
            }
            HttpResponse::Found()
                .insert_header((header::LOCATION, redirect_path))
                .finish()
        }
        Err(VerifyEmailError::InvalidOrExpired) => {
            redirect_login_with_error(&session, "Verification link is invalid or has expired.")
        }
        Err(VerifyEmailError::Database(err)) => {
            println!("Email verification error: {:?}", err);
            redirect_login_with_error(
                &session,
                "Unable to verify your email right now. Please try again.",
            )
        }
    }
}

#[post("/auth/resend-verification")]
pub async fn resend_verification_email(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let user_id = match session.get::<i32>("user_id").ok().flatten() {
        Some(user_id) => user_id,
        None => {
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
    };

    let user = match users::Entity::find_by_id(user_id).one(db.get_ref()).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            println!("Resend verification user lookup error: {:?}", err);
            return HttpResponse::InternalServerError().body("Unable to load your account.");
        }
    };

    if user.email_verified {
        let _ = session.insert("profile_success", "Your email is already verified.");
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/profile"))
            .finish();
    }

    let token = match create_email_verification_token(db.get_ref(), user.user_id).await {
        Ok(token) => token,
        Err(err) => {
            println!("Resend verification token error: {:?}", err);
            let _ = session.insert("profile_error", "Unable to create a verification email.");
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/profile"))
                .finish();
        }
    };

    match send_verification_email(&user.email, &token) {
        Ok(_) => {
            let _ = session.insert("profile_success", "Verification email sent.");
        }
        Err(err) => {
            println!("Resend verification email error: {}", err);
            let _ = session.insert("profile_error", "Unable to send verification email.");
        }
    }

    HttpResponse::Found()
        .insert_header((header::LOCATION, "/profile"))
        .finish()
}

#[get("/profile")]
pub async fn profile(db: web::Data<DatabaseConnection>, session: Session) -> impl Responder {
    let user_id = match session.get::<i32>("user_id").ok().flatten() {
        Some(user_id) => user_id,
        None => {
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
    };

    if session
        .get::<bool>("must_change_password")
        .ok()
        .flatten()
        .unwrap_or(false)
    {
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/change-password"))
            .finish();
    }

    let tera = Tera::new("../frontend/templates/**/*").expect("Failed to load templates");

    let mut context = build_page_context(&session);
    match users::Entity::find_by_id(user_id).one(db.get_ref()).await {
        Ok(Some(user)) => {
            context.insert("email_verified", &user.email_verified);
            let user_full_name = format!("{} {}", user.first_name.trim(), user.last_name.trim())
                .trim()
                .to_string();
            context.insert("user_full_name", &user_full_name);
            context.insert("has_password", &user.password_hash.is_some());
            let _ = session.insert("email_verified", user.email_verified);
        }
        Ok(None) => {
            session.purge();
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
        Err(err) => {
            println!("Profile user lookup error: {:?}", err);
            context.insert("error", "Unable to refresh your account details right now.");
        }
    }

    if let Ok(Some(success)) = session.get::<String>("profile_success") {
        context.insert("success", &success);
        session.remove("profile_success");
    }

    if let Ok(Some(error)) = session.get::<String>("profile_error") {
        context.insert("error", &error);
        session.remove("profile_error");
    }

    let html = tera
        .render("profile.html", &context)
        .expect("Failed to render profile.html");

    HttpResponse::Ok().content_type("text/html").body(html)
}

#[post("/logout")]
pub async fn logout(
    db: web::Data<DatabaseConnection>,
    req: HttpRequest,
    session: Session,
) -> impl Responder {
    if let Some(cookie) = req.cookie(REMEMBER_ME_COOKIE) {
        if let Err(err) = revoke_remember_me_token(db.get_ref(), cookie.value()).await {
            println!("Remember-me token revoke error: {:?}", err);
        }
    }

    session.purge();

    let mut response = HttpResponse::Found()
        .insert_header((header::LOCATION, "/login"))
        .finish();

    let forget_cookie = forget_remember_me_cookie();
    if let Err(err) = response.add_cookie(&forget_cookie) {
        println!("Remember-me cookie remove error: {:?}", err);
    }

    response
}

#[post("/profile/update-password")]
pub async fn update_password_submit(
    db: web::Data<DatabaseConnection>,
    session: Session,
    form: web::Form<ChangePasswordForm>,
) -> impl Responder {
    let user_id = match session.get::<i32>("user_id").ok().flatten() {
        Some(user_id) => user_id,
        None => {
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
    };

    let form = form.into_inner();
    if let Err(_) = form.validate() {
        let _ = session.insert("profile_error", "Please check your password details.");
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/profile"))
            .finish();
    }

    if form.new_password != form.confirm_password {
        let _ = session.insert("profile_error", "New password and confirm password do not match.");
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/profile"))
            .finish();
    }

    let user = match users::Entity::find_by_id(user_id).one(db.get_ref()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            session.purge();
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
        Err(err) => {
            println!("Update password user lookup error: {:?}", err);
            let _ = session.insert("profile_error", "Unable to load your account right now.");
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/profile"))
                .finish();
        }
    };

    let password_hash = match &user.password_hash {
        Some(h) => h.clone(),
        None => {
            let _ = session.insert("profile_error", "This account uses Google Sign-In and does not have a password.");
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/profile"))
                .finish();
        }
    };

    let parsed_hash = match PasswordHash::new(&password_hash) {
        Ok(hash) => hash,
        Err(err) => {
            println!("Update password hash parse error: {:?}", err);
            let _ = session.insert("profile_error", "Unable to verify your current password.");
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/profile"))
                .finish();
        }
    };

    if Argon2::default()
        .verify_password(form.current_password.as_bytes(), &parsed_hash)
        .is_err()
    {
        let _ = session.insert("profile_error", "Current password is incorrect.");
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/profile"))
            .finish();
    }

    let new_password_hash = match hash_password(form.new_password).await {
        Ok(hash) => hash,
        Err(message) => {
            let _ = session.insert("profile_error", &message);
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/profile"))
                .finish();
        }
    };

    let mut active_user = user.into_active_model();
    active_user.password_hash = Set(Some(new_password_hash));

    match active_user.update(db.get_ref()).await {
        Ok(_) => {
            let _ = session.insert("profile_success", "Password updated successfully.");
        }
        Err(err) => {
            println!("Update password save error: {:?}", err);
            let _ = session.insert("profile_error", "Unable to update your password right now.");
        }
    }

    HttpResponse::Found()
        .insert_header((header::LOCATION, "/profile"))
        .finish()
}

#[post("/profile/lecturer-signup")]
pub async fn lecturer_signup(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let user_id = match session.get::<i32>("user_id").ok().flatten() {
        Some(user_id) => user_id,
        None => {
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
    };

    let (_, current_role_names) = match load_user_roles(db.get_ref(), user_id).await {
        Ok(user_roles) => user_roles,
        Err(err) => {
            println!("User role lookup error: {:?}", err);
            let _ = session.insert(
                "profile_error",
                "Unable to check your account roles right now.",
            );
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/profile"))
                .finish();
        }
    };

    if current_role_names
        .iter()
        .any(|role_name| role_name == "Instructor")
    {
        let _ = session.insert(
            "profile_success",
            "Your account is already signed up as a lecturer.",
        );
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/profile"))
            .finish();
    }

    let instructor_role = match roles::Entity::find()
        .filter(roles::Column::RoleName.eq(roles::RoleName::Instructor))
        .one(db.get_ref())
        .await
    {
        Ok(Some(role)) => role,
        Ok(None) => {
            let _ = session.insert(
                "profile_error",
                "Lecturer role is not configured in the database.",
            );
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/profile"))
                .finish();
        }
        Err(err) => {
            println!("Instructor role lookup error: {:?}", err);
            let _ = session.insert(
                "profile_error",
                "Unable to process lecturer signup right now.",
            );
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/profile"))
                .finish();
        }
    };

    let new_user_role = user_roles::ActiveModel {
        user_id: Set(user_id),
        role_id: Set(instructor_role.role_id),
    };

    if let Err(err) = new_user_role.insert(db.get_ref()).await {
        println!("Lecturer signup insert error: {:?}", err);
        let _ = session.insert(
            "profile_error",
            "Unable to sign up as a lecturer right now.",
        );
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/profile"))
            .finish();
    }

    match load_user_roles(db.get_ref(), user_id).await {
        Ok((role_ids, role_names)) => {
            store_roles_in_session(&session, role_ids, role_names);
        }
        Err(err) => {
            println!("Role refresh error: {:?}", err);
        }
    }

    let _ = session.insert(
        "profile_success",
        "Lecturer signup complete. Your account now has lecturer access.",
    );

    HttpResponse::Found()
        .insert_header((header::LOCATION, "/profile"))
        .finish()
}


// ---------------------------------------------------------------------------
// Forgot password — step 1: request reset email
// ---------------------------------------------------------------------------

#[get("/forgot-password")]
pub async fn forgot_password_page(session: Session) -> impl Responder {
    if is_logged_in(&session) {
        return redirect_home();
    }

    let tera = Tera::new("../frontend/templates/**/*").expect("Failed to load templates");
    let mut context = build_page_context(&session);

    if let Ok(Some(success)) = session.get::<String>("flash_success") {
        context.insert("success", &success);
        session.remove("flash_success");
    }
    if let Ok(Some(error)) = session.get::<String>("flash_error") {
        context.insert("error", &error);
        session.remove("flash_error");
    }

    let html = tera
        .render("forgot_password.html", &context)
        .expect("Failed to render forgot_password.html");

    HttpResponse::Ok().content_type("text/html").body(html)
}

#[post("/forgot-password")]
pub async fn forgot_password_submit(
    db: web::Data<DatabaseConnection>,
    session: Session,
    form: web::Form<ForgotPasswordForm>,
) -> impl Responder {
    // Always show the same success message regardless of whether the email exists.
    // This prevents user enumeration attacks.
    let generic_success = "If that email address is registered, you will receive a password reset link shortly.";

    if let Err(_) = form.validate() {
        let _ = session.insert("flash_error", "Please enter a valid email address.");
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/forgot-password"))
            .finish();
    }

    let email = form.email.trim().to_lowercase();

    let user = match users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(db.get_ref())
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            // No account found — still show the generic message.
            let _ = session.insert("flash_success", generic_success);
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/forgot-password"))
                .finish();
        }
        Err(err) => {
            println!("Forgot-password user lookup error: {:?}", err);
            let _ = session.insert("flash_error", "Unable to process your request right now.");
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/forgot-password"))
                .finish();
        }
    };

    // Google-only accounts have no password to reset.
    if user.auth_provider == "google" && user.password_hash.is_none() {
        let _ = session.insert(
            "flash_success",
            "This account uses Google Sign-In. Please sign in with Google instead.",
        );
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/login"))
            .finish();
    }

    let token = match create_password_reset_token(db.get_ref(), user.user_id).await {
        Ok(t) => t,
        Err(err) => {
            println!("Create password reset token error: {:?}", err);
            let _ = session.insert("flash_error", "Unable to process your request right now.");
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/forgot-password"))
                .finish();
        }
    };

    match send_reset_email(&user.email, &token) {
        Ok(_) => {}
        Err(err) => {
            println!("Password reset email send error: {}", err);
            // Don't expose the send failure to the user.
        }
    }

    let _ = session.insert("flash_success", generic_success);
    HttpResponse::Found()
        .insert_header((header::LOCATION, "/forgot-password"))
        .finish()
}

// ---------------------------------------------------------------------------
// Forgot password — step 2: set new password using token
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ResetPasswordQuery {
    token: Option<String>,
}

#[get("/auth/reset-password")]
pub async fn reset_password_page(
    session: Session,
    query: web::Query<ResetPasswordQuery>,
) -> impl Responder {
    if is_logged_in(&session) {
        return redirect_home();
    }

    let token = match query.token.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => t.to_string(),
        None => {
            let _ = session.insert("flash_error", "Password reset link is missing a token.");
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/forgot-password"))
                .finish();
        }
    };

    let tera = Tera::new("../frontend/templates/**/*").expect("Failed to load templates");
    let mut context = build_page_context(&session);
    context.insert("token", &token);

    if let Ok(Some(error)) = session.get::<String>("flash_error") {
        context.insert("error", &error);
        session.remove("flash_error");
    }

    let html = tera
        .render("reset_password.html", &context)
        .expect("Failed to render reset_password.html");

    HttpResponse::Ok().content_type("text/html").body(html)
}

#[post("/auth/reset-password")]
pub async fn reset_password_submit(
    db: web::Data<DatabaseConnection>,
    session: Session,
    form: web::Form<ResetPasswordForm>,
) -> impl Responder {
    let form = form.into_inner();

    let render_error = |session: &Session, message: &str, token: &str| {
        let _ = session.insert("flash_error", message);
        HttpResponse::Found()
            .insert_header((
                header::LOCATION,
                format!("/auth/reset-password?token={}", token),
            ))
            .finish()
    };

    if let Err(_) = form.validate() {
        return render_error(&session, "Password must be between 8 and 128 characters.", &form.token);
    }

    if form.password != form.confirm_password {
        return render_error(&session, "Passwords do not match.", &form.token);
    }

    match reset_password_with_token(db.get_ref(), &form.token, form.password).await {
        Ok(_) => {
            let _ = session.insert(
                "flash_success",
                "Password reset successfully. You can now sign in.",
            );
            HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish()
        }
        Err(ResetPasswordError::InvalidOrExpired) => {
            render_error(&session, "This reset link is invalid or has expired. Please request a new one.", &form.token)
        }
        Err(ResetPasswordError::GoogleAccount) => {
            let _ = session.insert(
                "flash_error",
                "This account uses Google Sign-In and does not have a password.",
            );
            HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish()
        }
        Err(ResetPasswordError::Database(err)) => {
            println!("Reset password database error: {:?}", err);
            render_error(&session, "Unable to reset your password right now. Please try again.", &form.token)
        }
    }
}

#[get("/debug-session")]
async fn debug_session(session: Session) -> impl Responder {
    let user_id: Option<i32> = session.get("user_id").unwrap();
    let user_email: Option<String> = session.get("user_email").unwrap();
    let role_ids: Option<Vec<i32>> = session.get("role_ids").unwrap();
    let role_names: Option<Vec<String>> = session.get("role_names").unwrap();

    HttpResponse::Ok().json(serde_json::json!({
        "user_shouldntd": user_id,
        "user_email": user_email,
        "role_ids": role_ids,
        "role_names": role_names
    }))
}
