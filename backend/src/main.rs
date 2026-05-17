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
            .service(index)
            .service(Files::new("/static", "../frontend/static"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}