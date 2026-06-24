mod app_state;
mod config;
mod controller;
mod db;
mod entity;
mod models;
mod routes;
mod services;
mod ssr;
use actix_cors::Cors;
use actix_files::Files;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{
    App, HttpServer,
    cookie::SameSite,
    dev::ServiceResponse,
    http::{StatusCode, header},
    middleware::{ErrorHandlerResponse, ErrorHandlers, from_fn},
    web,
};
use app_state::AppState;
use db::connection::connect_db;
use services::csrf_service::{CsrfConfig, csrf_protection};
use services::login_rate_limit_service::LoginRateLimiter;
use services::remember_me_service::remember_me_middleware;

fn render_browser_500<B>(
    response: ServiceResponse<B>,
) -> actix_web::Result<ErrorHandlerResponse<B>> {
    let path = response.request().path();

    if path == "/api" || path.starts_with("/api/") {
        return Ok(ErrorHandlerResponse::Response(
            response.map_into_left_body(),
        ));
    }

    let (request, _) = response.into_parts();
    let error_response =
        ssr::pages::render_error_page("500.html", StatusCode::INTERNAL_SERVER_ERROR, None);

    Ok(ErrorHandlerResponse::Response(
        ServiceResponse::new(request, error_response.map_into_boxed_body()).map_into_right_body(),
    ))
}

fn redirect_browser_403<B>(
    response: ServiceResponse<B>,
) -> actix_web::Result<ErrorHandlerResponse<B>> {
    let path = response.request().path();

    if path == "/api" || path.starts_with("/api/") {
        return Ok(ErrorHandlerResponse::Response(
            response.map_into_left_body(),
        ));
    }

    let (request, _) = response.into_parts();
    let redirect_response = actix_web::HttpResponse::Found()
        .insert_header((header::LOCATION, "/"))
        .finish();

    Ok(ErrorHandlerResponse::Response(
        ServiceResponse::new(request, redirect_response.map_into_boxed_body())
            .map_into_right_body(),
    ))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::from_path(format!("{}/.env", env!("CARGO_MANIFEST_DIR")))
        .expect("Failed to load .env file");

    let db = connect_db().await;
    println!("Database connected!");

    let secret_key = config::session_key();
    let secure_cookies = config::is_production();
    let cors_allowed_origin = config::cors_allowed_origin();
    let csrf_config = web::Data::new(CsrfConfig::new(cors_allowed_origin.clone()));
    let login_rate_limiter = web::Data::new(LoginRateLimiter::default());
    let app_state = AppState::default();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&cors_allowed_origin)
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"])
            .allowed_headers(vec![header::ACCEPT, header::AUTHORIZATION, header::CONTENT_TYPE])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(actix_web::web::Data::new(db.clone()))
            .app_data(actix_web::web::Data::new(app_state.clone()))
            .app_data(csrf_config.clone())
            .app_data(login_rate_limiter.clone())
            .wrap(cors)
            .wrap(from_fn(csrf_protection))
            .wrap(
                ErrorHandlers::new()
                    .handler(StatusCode::FORBIDDEN, redirect_browser_403)
                    .handler(StatusCode::INTERNAL_SERVER_ERROR, render_browser_500),
            )
            .wrap(from_fn(remember_me_middleware))
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_http_only(true)
                    .cookie_same_site(SameSite::Lax)
                    .cookie_secure(secure_cookies)
                    .build(),
            )
            .service(
                web::scope("/api")
                    .configure(routes::assignment_routes::init)
                    .configure(routes::cloudinary::init)
                    .configure(routes::certificate_routes::init)
                    .configure(routes::payment_routes::init)
                    .configure(routes::course_routes::init)
                    .configure(routes::student_routes::init)
                    .configure(routes::enrollment_routes::init)
                    .configure(routes::organisation_routes::init)
                    .configure(routes::module_routes::init)
                    .configure(routes::module_content_routes::init)
                    .configure(routes::quiz_routes::init)
                    .configure(routes::quiz_attempts_routes::init)
                    .configure(routes::quiz_answers_routes::init)
                    .configure(routes::quiz_analytics_routes::init)
                    .configure(routes::grade_routes::init)
                    .configure(routes::submission_routes::init)
                    .configure(routes::viewer_routes::init),
            )
            .configure(ssr::pages::init)
            .configure(routes::user_routes::init)
            .configure(routes::admin_routes::init)
            .service(controller::organisation_controller::organisation_page)
            .service(controller::organisation_controller::organisation_signup_page)
            .service(controller::organisation_controller::organisation_signup_submit)
            .service(Files::new("/static", "../frontend/static"))
            .default_service(web::route().to(ssr::pages::not_found_page))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
