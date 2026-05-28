mod routes;
mod controller;
mod db; 
mod models;
mod entity;
mod services;
use db::connection::connect_db;
use actix_files::Files;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{cookie::Key, get, App, HttpResponse, HttpServer, Responder, web};
use actix_cors::Cors;
use tera::{Context, Tera};
#[get("/")]
async fn index() -> impl Responder {
    let tera = Tera::new("../frontend/templates/**/*")
        .expect("Failed to load templates");

    let context = Context::new();

    let html = tera
        .render("index.html", &context)
        .expect("Failed to render index.html");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}
#[get("/courses")]
async fn courses() -> impl Responder {
    render_page("courses.html")
}

#[get("/lessons")]
async fn lessons() -> impl Responder {
    render_page("lessons.html")
}

#[get("/assessments")]
async fn assessments() -> impl Responder {
    render_page("assessments.html")
}

#[get("/challenges")]
async fn challenges() -> impl Responder {
    render_page("challenges.html")
}

#[get("/certification")]
async fn certification() -> impl Responder {
    render_page("certification.html")
}

#[get("/projects")]
async fn projects() -> impl Responder {
    render_page("projects.html")
}

#[get("/downloads")]
async fn downloads() -> impl Responder {
    render_page("downloads.html")
}

#[get("/course/{course_id}")]
async fn course_details_page() -> impl Responder {
    render_page("course_details.html")
}

#[get("/module-content-page/{module_id}")]
async fn module_content_page() -> impl Responder {
    render_page("module_content.html")
}

pub fn render_page(template_name: &str) -> HttpResponse {
    let tera = Tera::new("../frontend/templates/**/*")
        .expect("Failed to load templates");

    let context = Context::new();

    let html = tera
        .render(template_name, &context)
        .expect("Failed to render template");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}
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
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    secret_key.clone(),
                )
                .cookie_secure(false)
                .build()
            )
            .configure(routes::assignment_routes::init)
            .configure(routes::cloudinary::init)
            .configure(routes::mailer::init)
            .configure(routes::payment_routes::init)
            .configure(routes::course_routes::init)
            .configure(routes::student_routes::init)
            .configure(routes::user_routes::init)
            .configure(routes::enrollment_routes::init)
            .configure(routes::module_routes::init)
            .configure(routes::module_content_routes::init)
            .service(index)
            .service(courses)
            .service(lessons)
            .service(assessments)
            .service(challenges)
            .service(certification)
            .service(projects)
            .service(downloads)
            .service(course_details_page)
            .service(module_content_page)
            .service(Files::new("/static", "../frontend/static").show_files_listing())
            
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}