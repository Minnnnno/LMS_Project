mod routes;
mod controller;
mod db; 
use db::connection::connect_db;
use actix_files::Files;
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
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
#[get("/login")]
async fn login() -> impl Responder {
    render_page("login.html")
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

fn render_page(template_name: &str) -> HttpResponse {
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
    let db_pool = connect_db().await;
    println!("Database connected!");
    HttpServer::new(move || {
    App::new()
        .app_data(actix_web::web::Data::new(db_pool.clone()))
        .wrap(Cors::permissive())
        .configure(routes::cloudinary::init)
        .configure(routes::mailer::init)
        .service(index)
        .service(login)
        .service(courses)
        .service(lessons)
        .service(assessments)
        .service(challenges)
        .service(certification)
        .service(projects)
        .service(downloads)
        .service(Files::new("/static", "../frontend/static"))
})
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}