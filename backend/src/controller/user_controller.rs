use crate::entity::{roles, user_roles, users};
use crate::models::user::{LoginForm, RegisterForm};
use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, get, post, web};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, Set, TransactionTrait,
};
use serde::Deserialize;
use std::env;

use crate::ssr::pages::{build_page_context, render_page};
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{PasswordHash, PasswordVerifier},
    password_hash::{SaltString, rand_core::OsRng},
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

fn redirect_home() -> HttpResponse {
    HttpResponse::Found()
        .insert_header((header::LOCATION, "/"))
        .finish()
}

fn is_logged_in(session: &Session) -> bool {
    session.get::<i32>("user_id").ok().flatten().is_some()
}

fn role_name_to_string(role_name: roles::RoleName) -> String {
    match role_name {
        roles::RoleName::LmsAdmin => "LMS Admin",
        roles::RoleName::OrganisationAdmin => "Organisation Admin",
        roles::RoleName::Instructor => "Instructor",
        roles::RoleName::Student => "Student",
    }
    .to_string()
}

async fn load_user_roles(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<(Vec<i32>, Vec<String>), DbErr> {
    let user_role_rows = user_roles::Entity::find()
        .filter(user_roles::Column::UserId.eq(user_id))
        .all(db)
        .await?;

    let role_ids: Vec<i32> = user_role_rows
        .iter()
        .map(|user_role| user_role.role_id)
        .collect();

    if role_ids.is_empty() {
        return Ok((role_ids, Vec::new()));
    }

    let role_names = roles::Entity::find()
        .filter(roles::Column::RoleId.is_in(role_ids.clone()))
        .all(db)
        .await?
        .into_iter()
        .map(|role| role_name_to_string(role.role_name))
        .collect::<Vec<String>>();

    Ok((role_ids, role_names))
}

fn store_roles_in_session(session: &Session, role_ids: Vec<i32>, role_names: Vec<String>) {
    if let Err(err) = session.insert("role_ids", role_ids) {
        println!("Session insert error: {:?}", err);
    }
    if let Err(err) = session.insert("role_names", role_names) {
        println!("Session insert error: {:?}", err);
    }
}

async fn assign_role_to_user<C>(
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

    let new_user_role = user_roles::ActiveModel {
        user_id: Set(user_id),
        role_id: Set(role.role_id),
    };

    new_user_role.insert(db).await?;

    Ok(())
}

fn redirect_login_with_error(session: &Session, error_message: &str) -> HttpResponse {
    if let Err(err) = session.insert("flash_error", error_message) {
        println!("Session flash insert error: {:?}", err);
    }

    HttpResponse::Found()
        .insert_header((header::LOCATION, "/login"))
        .finish()
}

fn google_client_id() -> Result<String, String> {
    env::var("GOOGLE_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Google login is not configured. Missing GOOGLE_CLIENT_ID.".to_string())
}

fn google_client_secret() -> Result<String, String> {
    env::var("GOOGLE_CLIENT_SECRET")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Google login is not configured. Missing GOOGLE_CLIENT_SECRET.".to_string())
}

fn google_redirect_uri() -> String {
    env::var("GOOGLE_REDIRECT_URI")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8080/auth/google/callback".to_string())
}

async fn hash_password(password: String) -> Result<String, String> {
    let password_hash_result = actix_web::rt::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);

        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    })
    .await;

    match password_hash_result {
        Ok(Ok(hash)) => Ok(hash),
        Ok(Err(err)) => {
            println!("Password hashing error: {:?}", err);
            Err("Failed to process password.".to_string())
        }
        Err(err) => {
            println!("Hashing task error: {:?}", err);
            Err("Failed to process password.".to_string())
        }
    }
}

async fn sign_user_into_session(
    db: &DatabaseConnection,
    session: &Session,
    user: &users::Model,
) -> Result<(), String> {
    let (role_ids, role_names) = load_user_roles(db, user.user_id).await.map_err(|err| {
        println!("User role lookup error: {:?}", err);
        "Unable to process login at this time.".to_string()
    })?;

    session.renew();

    if let Err(err) = session.insert("user_id", user.user_id) {
        println!("Session insert error: {:?}", err);
    }
    if let Err(err) = session.insert("user_email", user.email.clone()) {
        println!("Session insert error: {:?}", err);
    }
    store_roles_in_session(session, role_ids, role_names);

    Ok(())
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

#[get("/login")]
pub async fn login(session: Session) -> impl Responder {
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
        .render("login.html", &context)
        .expect("Failed to render login.html");

    HttpResponse::Ok().content_type("text/html").body(html)
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

    if let Err(err) = txn.commit().await {
        println!("Registration commit error: {:?}", err);
        return render_register_error(
            "Unable to register your account right now.",
            &first_name,
            &last_name,
            &email,
        );
    }

    if let Err(err) = session.insert(
        "flash_success",
        "User registered successfully. Please log in.",
    ) {
        println!("Session flash insert error: {:?}", err);
    }

    HttpResponse::Found()
        .insert_header((header::LOCATION, "/login"))
        .finish()
}

#[post("/login")]
pub async fn login_submit(
    db: web::Data<DatabaseConnection>,
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
    store_roles_in_session(&session, role_ids, role_names);

    println!("Login successful. Stored user_id: {}", user.user_id);

    redirect_home()
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
        Ok(Some(user)) => user,
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

    redirect_home()
}

#[get("/profile")]
pub async fn profile(session: Session) -> impl Responder {
    if !is_logged_in(&session) {
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/login"))
            .finish();
    }

    let tera = Tera::new("../frontend/templates/**/*").expect("Failed to load templates");

    let mut context = build_page_context(&session);
    let role_names = session
        .get::<Vec<String>>("role_names")
        .ok()
        .flatten()
        .unwrap_or_default();
    let can_signup_as_lecturer = !role_names
        .iter()
        .any(|role_name| role_name == "Instructor");
    context.insert("can_signup_as_lecturer", &can_signup_as_lecturer);

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
pub async fn logout(session: Session) -> impl Responder {
    session.purge();

    HttpResponse::Found()
        .insert_header((header::LOCATION, "/login"))
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
