mod routes;
mod controller;
mod db; 
mod models;
mod entity;
mod services;
mod ssr;
use db::connection::connect_db;
use actix_files::Files;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{cookie::Key, middleware::from_fn, web, App, HttpServer};
use actix_cors::Cors;
use services::remember_me_service::remember_me_middleware;
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::from_path(format!("{}/.env", env!("CARGO_MANIFEST_DIR")))
        .expect("Failed to load .env file");

    let db = connect_db().await;
    println!("Database connected!");

    let secret_key = Key::generate();

    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(db.clone()))
            .wrap(Cors::permissive())
            .wrap(from_fn(remember_me_middleware))
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    secret_key.clone(),
                )
                .cookie_secure(false)
                .build()
            )
            .service(
                web::scope("/api")
                    .configure(routes::assignment_routes::init)
                    .configure(routes::cloudinary::init)
                    .configure(routes::mailer::init)
                    .configure(routes::payment_routes::init)
                    .configure(routes::course_routes::init)
                    .configure(routes::student_routes::init)
                    .configure(routes::enrollment_routes::init)
                    .configure(routes::organisation_routes::init)
                    .configure(routes::module_routes::init)
                    .configure(routes::module_content_routes::init)
                    .configure(routes::quiz_routes::init)
                    .configure(routes::quiz_questions_routes::init)
                    .configure(routes::quiz_options_routes::init)
                    .configure(routes::quiz_attempts_routes::init)
                        .configure(routes::quiz_answers_routes::init)
            )
            .configure(ssr::pages::init)
            .configure(routes::user_routes::init)
                    .configure(routes::admin_routes::init)
            .service(controller::organisation_controller::organisation_page)
            .service(controller::organisation_controller::organisation_signup_page)
            .service(controller::organisation_controller::organisation_signup_submit)
            .service(Files::new("/static", "../frontend/static").show_files_listing())
            
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
