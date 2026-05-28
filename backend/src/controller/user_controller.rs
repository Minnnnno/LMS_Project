use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, web, post};
use actix_web::http::header;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set};
use crate::entity::{roles, user_roles, users};
use crate::models::user::{LoginForm, RegisterForm};

use tera::{Context, Tera};
use validator::Validate;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2, PasswordHasher,
};
use crate::{build_page_context, render_page};

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

    let role_ids: Vec<i32> = user_role_rows.iter().map(|user_role| user_role.role_id).collect();

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

fn render_register_error(error_message: &str, first_name: &str, last_name: &str, email: &str) -> HttpResponse {
    let tera = Tera::new("../frontend/templates/**/*")
        .expect("Failed to load templates");

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
    let tera = Tera::new("../frontend/templates/**/*")
        .expect("Failed to load templates");

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

    let tera = Tera::new("../frontend/templates/**/*")
        .expect("Failed to load templates");

    let mut context = build_page_context(&session);
    if let Ok(Some(success)) = session.get::<String>("flash_success") {
        context.insert("success", &success);
        session.remove("flash_success");
    }

    let html = tera
        .render("login.html", &context)
        .expect("Failed to render login.html");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
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

    //form valdation successful
    // salt and hash password
    let password = form.password.clone();

    let password_hash_result = actix_web::rt::task::spawn_blocking(move || {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
    })
    .await;

    let password_hash = match password_hash_result {
        Ok(Ok(hash)) => hash,

        Ok(Err(err)) => {
            println!("Password hashing error: {:?}", err);
            return render_register_error(
                "Failed to process password.",
                &form.first_name,
                &form.last_name,
                &form.email,
            );
        }

        Err(err) => {
            println!("Hashing task error: {:?}", err);
            return render_register_error(
                "Failed to process password.",
                &form.first_name,
                &form.last_name,
                &form.email,
            );
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
        password_hash: Set(password_hash),

        // org_id is not set here.
        // This lets the database use its default value.
        ..Default::default()
    };

    let response = match new_user.insert(db.get_ref()).await {
        Ok(_) => {
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

    response
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
        return render_login_error(
            "Please enter a valid email and password.",
            &form.email,
        );
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
            return render_login_error(
                "Incorrect email or password.",
                &email,
            );
        }
        Err(err) => {
            println!("Login lookup error: {:?}", err);
            return render_login_error(
                "Unable to process login at this time.",
                &email,
            );
        }
    };

    let parsed_hash = match PasswordHash::new(&user.password_hash) {
        Ok(hash) => hash,
        Err(err) => {
            println!("Password hash parse error: {:?}", err);
            return render_login_error(
                "Incorrect email or password.",
                &email,
            );
        }
    };

    if Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_err() {
        return render_login_error(
            "Incorrect email or password.",
            &email,
        );
    }

    let (role_ids, role_names) = match load_user_roles(db.get_ref(), user.user_id).await {
        Ok(user_roles) => user_roles,
        Err(err) => {
            println!("User role lookup error: {:?}", err);
            return render_login_error(
                "Unable to process login at this time.",
                &email,
            );
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

#[get("/profile")]
pub async fn profile(session: Session) -> impl Responder {
    if !is_logged_in(&session) {
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/login"))
            .finish();
    }

    let tera = Tera::new("../frontend/templates/**/*")
        .expect("Failed to load templates");

    let mut context = build_page_context(&session);
    let role_names = session
        .get::<Vec<String>>("role_names")
        .ok()
        .flatten()
        .unwrap_or_default();
    let can_signup_as_lecturer = !role_names.iter().any(|role_name| role_name == "Student")
        && !role_names.iter().any(|role_name| role_name == "Instructor");
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

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
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

    if current_role_names.iter().any(|role_name| role_name == "Student") {
        let _ = session.insert(
            "profile_error",
            "Student accounts cannot sign up as lecturers.",
        );
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/profile"))
            .finish();
    }

    if current_role_names.iter().any(|role_name| role_name == "Instructor") {
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

