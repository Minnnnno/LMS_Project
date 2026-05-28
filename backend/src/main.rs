mod routes;
mod controller;
mod db; 
mod models;
mod entity;
mod services;
use db::connection::connect_db;
use actix_files::Files;
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_web::{cookie::Key, get, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use tera::{Context, Tera};
#[get("/")]
async fn index(session: Session) -> impl Responder {
    let tera = Tera::new("../frontend/templates/**/*")
        .expect("Failed to load templates");

    let context = build_page_context(&session);

    let html = tera
        .render("index.html", &context)
        .expect("Failed to render index.html");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}
#[get("/courses")]
async fn courses(session: Session) -> impl Responder {
    render_page("courses.html", &session)
}

#[get("/lessons")]
async fn lessons(session: Session) -> impl Responder {
    render_page("lessons.html", &session)
}

#[get("/assessments")]
async fn assessments(session: Session) -> impl Responder {
    render_page("assessments.html", &session)
}

#[get("/challenges")]
async fn challenges(session: Session) -> impl Responder {
    render_page("challenges.html", &session)
}

#[get("/certification")]
async fn certification(session: Session) -> impl Responder {
    render_page("certification.html", &session)
}

#[get("/projects")]
async fn projects(session: Session) -> impl Responder {
    render_page("projects.html", &session)
}

#[get("/downloads")]
async fn downloads(session: Session) -> impl Responder {
    render_page("downloads.html", &session)
}

#[get("/course-page/{course_id}")]
async fn course_details_page(session: Session) -> impl Responder {
    render_page("course_details.html", &session)
}

#[get("/module-content-page/{module_id}")]
async fn module_content_page(session: Session) -> impl Responder {
    render_page("module_content.html",&session)
}

#[get("/pdf-viewer-page")]
async fn pdf_viewer_page(session: Session) -> impl Responder {
    render_page("pdf_viewer.html", &session)
}

pub fn build_page_context(session: &Session) -> Context {
    let mut context: Context = Context::new();

    if let Ok(Some(user_id)) = session.get::<i32>("user_id") {
        context.insert("is_logged_in", &true);
        context.insert("user_id", &user_id);
    } else {
        context.insert("is_logged_in", &false);
    }

    if let Ok(Some(user_email)) = session.get::<String>("user_email") {
        context.insert("user_email", &user_email);
    }

    let role_names: Vec<String> = session
        .get::<Vec<String>>("role_names")
        .ok()
        .flatten()
        .unwrap_or_default();

    context.insert("role_names", &role_names);

    let role_ids: Vec<i32> = session
        .get::<Vec<i32>>("role_ids")
        .ok()
        .flatten()
        .unwrap_or_default();

    context.insert("role_ids", &role_ids);

    context
}

pub fn render_page(template_name: &str, session: &Session) -> HttpResponse {
    let tera: Tera = Tera::new(("../frontend/templates/**/*"))
        .expect("Failed to load templates");

    let context = build_page_context(session);

    let html: String = tera
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
            .configure(routes::organisation_routes::init)
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
            .service(pdf_viewer_page)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
