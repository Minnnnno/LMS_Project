use actix_session::Session;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use sea_orm::DatabaseConnection;

use crate::services::payment_service;

#[get("/payment-success")]
pub async fn payment_success() -> impl Responder {
    HttpResponse::Ok().body("Payment completed. Waiting for webhook confirmation.")
}

#[get("/payment-cancelled")]
pub async fn payment_cancelled() -> impl Responder {
    HttpResponse::Ok().body("Payment was cancelled.")
}

#[post("/courses/{course_id}/checkout")]
pub async fn create_checkout_session(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    session: Session,
) -> impl Responder {
    payment_service::create_checkout_session(db.get_ref(), &session, path.into_inner()).await
}

#[post("/stripe/webhook")]
pub async fn stripe_webhook(
    db: web::Data<DatabaseConnection>,
    req: HttpRequest,
    body: String,
) -> impl Responder {
    println!("Webhook endpoint hit");

    let stripe_signature = match req.headers().get("stripe-signature") {
        Some(signature) => match signature.to_str() {
            Ok(value) => value,
            Err(_) => return HttpResponse::BadRequest().body("Invalid stripe-signature header"),
        },
        None => return HttpResponse::BadRequest().body("Missing stripe-signature header"),
    };

    payment_service::handle_stripe_webhook(db.get_ref(), stripe_signature, &body).await
}
