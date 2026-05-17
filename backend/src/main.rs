mod routes;
mod controller;
mod db; 
use db::connection::connect_db;
use actix_web::{App, HttpServer};
use actix_cors::Cors;

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
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}