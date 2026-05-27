use actix_session::Session;
use actix_web::{HttpResponse, Responder, get, web, post};
use actix_web::http::header;
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait, QueryFilter, ColumnTrait};
use crate::entity::users;
use crate::models::user::{LoginForm, RegisterForm};

use tera::{Context, Tera};
use validator::Validate;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2, PasswordHasher,
};
use crate::render_page;

fn render_register_error(error_message: &str, first_name: &str, last_name: &str, email: &str) -> HttpResponse {
    let tera = Tera::new("../frontend/templates/**/*")
        .expect("Failed to load templates");

    let mut context = Context::new();
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
pub async fn login() -> impl Responder {
    render_page("login.html")
}



#[get("/register")]
pub async fn register() -> impl Responder {
    render_page("register.html")
}


#[post("/register")]
pub async fn register_submit(
    db: web::Data<DatabaseConnection>,
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
        Ok(_) => HttpResponse::Ok().body("User registered successfully"),
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

    if let Err(err) = session.insert("user_id", user.user_id) {
        println!("Session insert error: {:?}", err);
    }
    if let Err(err) = session.insert("user_email", user.email.clone()) {
        println!("Session insert error: {:?}", err);
    }


    println!("Login successful. Stored user_id: {}", user.user_id);

    

    session.renew();

    HttpResponse::Found()
        .insert_header((header::LOCATION, "/"))
        .finish()
}

