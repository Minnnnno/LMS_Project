use actix_session::Session;
use actix_web::{rt::time::sleep, HttpResponse};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use serde::Serialize;
use std::time::Duration;
use stripe::{
    CheckoutSession, CheckoutSessionMode, CheckoutSessionPaymentStatus, Client,
    CreateCheckoutSession, CreateCheckoutSessionLineItems,
    CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData,
    CreateCheckoutSessionPaymentIntentData, Currency, EventObject, EventType, Webhook,
};

use crate::entity::{courses, enrollments, payments};

const PENDING_PAYMENT_TTL_SECONDS: u64 = 30 * 60;
const STRIPE_CHECKOUT_EXPIRY_SECONDS: i64 = 30 * 60;

#[derive(Serialize)]
struct CheckoutResponse {
    message: String,
    payment_id: i32,
    checkout_session_id: String,
    checkout_url: String,
}

fn schedule_pending_payment_cleanup(db: DatabaseConnection, payment_id: i32) {
    actix_web::rt::spawn(async move {
        sleep(Duration::from_secs(PENDING_PAYMENT_TTL_SECONDS)).await;

        match payments::Entity::delete_many()
            .filter(payments::Column::PaymentId.eq(payment_id))
            .filter(payments::Column::PaymentStatus.eq("PENDING"))
            .exec(&db)
            .await
        {
            Ok(result) if result.rows_affected > 0 => {
                println!("Deleted expired pending payment {}", payment_id);
            }
            Ok(_) => {
                println!("Payment {} was no longer pending, cleanup skipped", payment_id);
            }
            Err(err) => {
                println!("Failed to delete expired pending payment {}: {}", payment_id, err);
            }
        }
    });
}

pub async fn create_checkout_session(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> HttpResponse {
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(id)) => id,
        Ok(None) => return HttpResponse::Unauthorized().body("Please log in before buying a course"),
        Err(err) => return HttpResponse::InternalServerError().body(format!("Session error: {}", err)),
    };

    match enrollments::Entity::find()
        .filter(enrollments::Column::UserId.eq(user_id))
        .filter(enrollments::Column::CourseId.eq(course_id))
        .one(db)
        .await
    {
        Ok(Some(_)) => return HttpResponse::BadRequest().body("User is already enrolled in this course"),
        Ok(None) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error checking enrollment: {}", err));
        }
    }

    let course = match courses::Entity::find_by_id(course_id).one(db).await {
        Ok(Some(course)) => course,
        Ok(None) => return HttpResponse::NotFound().body("Course not found"),
        Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    };

    if !course.is_paid.unwrap_or(false) {
        return HttpResponse::BadRequest().body("This course is free, no checkout session needed");
    }

    let payment = payments::ActiveModel {
        user_id: Set(user_id),
        course_id: Set(course.course_id),
        provider: Set("stripe".to_string()),
        amount_cents: Set(course.price_cents.unwrap_or(0)),
        currency: Set(course.currency.clone().unwrap_or("SGD".to_string())),
        payment_status: Set("PENDING".to_string()),
        ..Default::default()
    };

    let inserted_payment = match payment.insert(db).await {
        Ok(inserted_payment) => inserted_payment,
        Err(err) => return HttpResponse::InternalServerError().body(format!("Insert payment error: {}", err)),
    };

    let stripe_secret_key = match std::env::var("STRIPE_SECRET_KEY") {
        Ok(key) => key,
        Err(_) => return HttpResponse::InternalServerError().body("Stripe secret key not set in .env"),
    };

    let frontend_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    let stripe_client = Client::new(stripe_secret_key);
    let success_url = format!("{}/", frontend_url);
    let cancel_url = format!(
        "{}/courses?payment=cancelled&course_id={}&payment_id={}",
        frontend_url, course.course_id, inserted_payment.payment_id
    );

    let stripe_currency = match course
        .currency
        .clone()
        .unwrap_or("SGD".to_string())
        .to_lowercase()
        .as_str()
    {
        "sgd" => Currency::SGD,
        _ => return HttpResponse::BadRequest().body("Unsupported currency"),
    };

    let mut session_params = CreateCheckoutSession::new();
    session_params.success_url = Some(success_url.as_str());
    session_params.cancel_url = Some(cancel_url.as_str());
    session_params.expires_at = Some(
        (Utc::now() + chrono::Duration::seconds(STRIPE_CHECKOUT_EXPIRY_SECONDS)).timestamp(),
    );
    session_params.mode = Some(CheckoutSessionMode::Payment);
    session_params.line_items = Some(vec![CreateCheckoutSessionLineItems {
        quantity: Some(1),
        price_data: Some(CreateCheckoutSessionLineItemsPriceData {
            currency: stripe_currency,
            unit_amount: Some(course.price_cents.unwrap_or(0) as i64),
            product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                name: course.name.clone().unwrap_or("Course".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }]);

    let metadata = std::collections::HashMap::from([
        ("payment_id".to_string(), inserted_payment.payment_id.to_string()),
        ("user_id".to_string(), user_id.to_string()),
        ("course_id".to_string(), course.course_id.to_string()),
    ]);

    session_params.metadata = Some(metadata.clone());
    session_params.payment_intent_data = Some(CreateCheckoutSessionPaymentIntentData {
        metadata: Some(metadata),
        ..Default::default()
    });

    let checkout_session = match CheckoutSession::create(&stripe_client, session_params).await {
        Ok(session) => session,
        Err(err) => return HttpResponse::InternalServerError().body(format!("Stripe API error: {}", err)),
    };

    let checkout_session_id = checkout_session.id.to_string();
    let checkout_url = match checkout_session.url {
        Some(url) => url,
        None => return HttpResponse::InternalServerError().body("Stripe API error: no checkout URL returned"),
    };

    let payment_id = inserted_payment.payment_id;
    let mut active_payment = inserted_payment.into_active_model();
    active_payment.checkout_session_id = Set(Some(checkout_session_id.clone()));

    match active_payment.update(db).await {
        Ok(_) => {
            schedule_pending_payment_cleanup(db.clone(), payment_id);
            HttpResponse::Ok().json(CheckoutResponse {
                message: "Checkout session created successfully".to_string(),
                payment_id,
                checkout_session_id,
                checkout_url,
            })
        }
        Err(err) => {
            HttpResponse::InternalServerError()
                .body(format!("Failed to update checkout session id: {}", err))
        }
    }
}

pub async fn handle_stripe_webhook(
    db: &DatabaseConnection,
    stripe_signature: &str,
    body: &str,
) -> HttpResponse {
    let webhook_secret = match std::env::var("STRIPE_WEBHOOK_SECRET") {
        Ok(secret) => secret,
        Err(_) => return HttpResponse::InternalServerError().body("Stripe webhook secret missing in .env"),
    };

    let event = match Webhook::construct_event(body, stripe_signature, &webhook_secret) {
        Ok(event) => event,
        Err(err) => return HttpResponse::BadRequest().body(format!("Webhook verification failed: {}", err)),
    };

    println!("Webhook event type: {:?}", event.type_);

    match event.type_ {
        EventType::CheckoutSessionCompleted => handle_checkout_session_completed(db, event).await,
        EventType::PaymentIntentSucceeded => handle_payment_intent_succeeded(db, event).await,
        _ => {
            println!("Event ignored");
            HttpResponse::Ok().body("Event ignored")
        }
    }
}

async fn handle_checkout_session_completed(
    db: &DatabaseConnection,
    event: stripe::Event,
) -> HttpResponse {
    let session = match event.data.object {
        EventObject::CheckoutSession(session) => session,
        _ => return HttpResponse::BadRequest().body("Event object is not a CheckoutSession"),
    };

    if session.payment_status != CheckoutSessionPaymentStatus::Paid {
        return HttpResponse::Ok().body("Checkout session completed but payment is not paid");
    }

    let metadata = match session.metadata {
        Some(metadata) => metadata,
        None => return HttpResponse::BadRequest().body("No metadata found in checkout session"),
    };

    let payment_id = match metadata.get("payment_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => return HttpResponse::BadRequest().body("Missing or invalid payment_id metadata"),
    };
    let user_id = match metadata.get("user_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => return HttpResponse::BadRequest().body("Missing or invalid user_id metadata"),
    };
    let course_id = match metadata.get("course_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => return HttpResponse::BadRequest().body("Missing or invalid course_id metadata"),
    };

    let payment_ref = session
        .payment_intent
        .map(|payment_intent| payment_intent.id().to_string());

    fulfill_payment(db, payment_id, user_id, course_id, payment_ref).await
}

async fn handle_payment_intent_succeeded(
    db: &DatabaseConnection,
    event: stripe::Event,
) -> HttpResponse {
    let payment_intent = match event.data.object {
        EventObject::PaymentIntent(payment_intent) => payment_intent,
        _ => return HttpResponse::BadRequest().body("Event object is not a PaymentIntent"),
    };

    let metadata = payment_intent.metadata;
    let payment_id = match metadata.get("payment_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => return HttpResponse::BadRequest().body("Missing or invalid payment_id metadata"),
    };
    let user_id = match metadata.get("user_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => return HttpResponse::BadRequest().body("Missing or invalid user_id metadata"),
    };
    let course_id = match metadata.get("course_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => return HttpResponse::BadRequest().body("Missing or invalid course_id metadata"),
    };

    let payment_ref = Some(payment_intent.id.to_string());
    fulfill_payment(db, payment_id, user_id, course_id, payment_ref).await
}

async fn fulfill_payment(
    db: &DatabaseConnection,
    payment_id: i32,
    user_id: i32,
    course_id: i32,
    payment_ref: Option<String>,
) -> HttpResponse {
    let existing_payment = match payments::Entity::find_by_id(payment_id).one(db).await {
        Ok(Some(payment)) => payment,
        Ok(None) => return HttpResponse::NotFound().body("Payment row not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding payment: {}", err));
        }
    };

    if existing_payment.payment_status == "SUCCEEDED" {
        return HttpResponse::Ok().body("Payment already fulfilled");
    }

    let mut active_payment = existing_payment.into_active_model();
    active_payment.payment_status = Set("SUCCEEDED".to_string());
    active_payment.payment_ref = Set(payment_ref);
    active_payment.paid_at = Set(Some(Utc::now()));

    if let Err(err) = active_payment.update(db).await {
        return HttpResponse::InternalServerError()
            .body(format!("Database error updating payment: {}", err));
    }

    let enrollment = enrollments::ActiveModel {
        user_id: Set(user_id),
        course_id: Set(course_id),
        ..Default::default()
    };

    match enrollment.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("Payment succeeded and enrollment created"),
        Err(err) => {
            println!("Enrollment insert issue: {}", err);
            HttpResponse::Ok().body(format!("Payment updated, enrollment may already exist: {}", err))
        }
    }
}
