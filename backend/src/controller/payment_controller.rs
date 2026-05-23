use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};

use sea_orm::{
    DatabaseConnection, EntityTrait, ActiveModelTrait, IntoActiveModel,
    Set, ColumnTrait, QueryFilter,
};

use serde::Serialize;

use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData,
    CreateCheckoutSessionPaymentIntentData,
    Currency,
    EventObject, EventType, Webhook, CheckoutSessionPaymentStatus,
};

use crate::entity::{courses, payments, enrollments}; //use course, payment and enrollment table entity

use chrono::Utc;


//struct for converting struct into json response
#[derive(Serialize)]
struct CheckoutResponse {
    message: String,
    payment_id: i32,
    checkout_session_id: String,
    checkout_url: String,
}

//payment success page
#[get("/payment-success")]
pub async fn payment_success() -> impl Responder {
    HttpResponse::Ok().body("Payment completed. Waiting for webhook confirmation.")
}


//payment cancelled page
#[get("/payment-cancelled")]
pub async fn payment_cancelled() -> impl Responder {
    HttpResponse::Ok().body("Payment was cancelled.")
}




#[post("/courses/{course_id}/checkout")]
pub async fn create_checkout_session(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    let course_id = path.into_inner();


    //temp hardcoded id for testing, replaced once login authentication is complete
    let user_id = 1;


    //check if user is already enrolled in the course
    let existing_enrollment = enrollments::Entity::find()
        .filter(enrollments::Column::UserId.eq(user_id))
        .filter(enrollments::Column::CourseId.eq(course_id))
        .one(db.get_ref())
        .await;

    match existing_enrollment {
        Ok(Some(_)) => {
            return HttpResponse::BadRequest()
                .body("User is already enrolled in this course");
        }
        Ok(None) => {
            //not enrolled yet, continue payment flow
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error checking enrollment: {}", err));
            
        }
    }

    //search for course in database
    let course_result = courses::Entity::find_by_id(course_id)
        .one(db.get_ref()) //.one means return either one course or nothing, .get)_ref() is used to get the database connection from the web::Data wrapper
        .await;


    //handles results from database query, if course is found, continue with checkout session creation, otherwise return  error response
    let course = match course_result {
        Ok(Some(course)) => course,
        Ok(None) => {
            return HttpResponse::NotFound()
                .body("Course not found")
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err))
        }
    };

    // if course is a free course
    if !course.is_paid {
        return HttpResponse::BadRequest()
            .body("This course is free, no checkout session needed")
    }
    

    //SeaORM active model
    //create new payment record with status "PENDING" in database
    let payment = payments::ActiveModel {
        user_id: Set(user_id),
        course_id: Set(course.course_id),
        provider: Set("stripe".to_string()), //hardcoded for now, will be dynamic once more payment providers are added
        amount_cents: Set(course.price_cents),
        currency: Set(course.currency.clone()),   //.clone because still need when creating stripe checkout session
        payment_status: Set("PENDING".to_string()),
        ..Default::default() //fill in the rest of the fields with default values
    };
    

    //insert new payment record into database
    let inserted_payment = match payment.insert(db.get_ref()).await {
    Ok(inserted_payment) => inserted_payment,
    Err(err) => {
        return HttpResponse::InternalServerError()
            .body(format!("Insert payment error: {}", err));
    }
};




    //stripe key from .env file, return error response if not set
    let stripe_secret_key = match std::env::var("STRIPE_SECRET_KEY") {
        Ok(key) => key,
        Err(_) => {
            return HttpResponse::InternalServerError()
                .body("Stripe secret key not set in .env");
        }
    };

    //frontend url from .env file for flexibility, default to localhost if not set
    let frontend_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    //create stripe client
    let stripe_client = Client::new(stripe_secret_key);

    //success url for payment completion
    let success_url = format!(
        "{}/payment-success?payment_id={}",
        frontend_url,
        inserted_payment.payment_id
    );

    //cancel url for payment cancellation
    let cancel_url = format!(
        "{}/payment-cancelled?payment_id={}",
        frontend_url,
        inserted_payment.payment_id
    );

    //convert database currency string into Stripe Currency enum
    let stripe_currency = match course.currency.to_lowercase().as_str() {
        "sgd" => Currency::SGD,
        //to add more supported currency in the future..
        _ => {
            return HttpResponse::BadRequest()
                .body("Unsupported currency");
        }
    };

    //stripe checkout session creation parameters
    let mut session_params = CreateCheckoutSession::new();

    session_params.success_url = Some(success_url.as_str());
    session_params.cancel_url = Some(cancel_url.as_str());

    //setting of payment mode
    session_params.mode = Some(CheckoutSessionMode::Payment);

    //adding of line items to the checkout session.
    session_params.line_items = Some(vec![
        CreateCheckoutSessionLineItems {
            quantity: Some(1), //quantity only 1
            price_data: Some(CreateCheckoutSessionLineItemsPriceData {
                currency: stripe_currency,
                unit_amount: Some(course.price_cents as i64), //price in cents
                product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                    name: course.name.clone(), //course name as product name in stripe
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    ]);

    let metadata = std::collections::HashMap::from([
    ("payment_id".to_string(), inserted_payment.payment_id.to_string()),
    ("user_id".to_string(), user_id.to_string()),
    ("course_id".to_string(), course.course_id.to_string()),
    ]);

    //metadata attached to checkout session
    session_params.metadata = Some(metadata.clone());

    //metadata attached to the PaymentIntent as well
    session_params.payment_intent_data = Some(CreateCheckoutSessionPaymentIntentData {
        metadata: Some(metadata),
        ..Default::default()
    });

    //calling stripe api to create checkout session, return error response if api call fails
    let checkout_session = match CheckoutSession::create(&stripe_client, session_params).await {
        Ok(session) => session,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Stripe API error: {}", err));
        }
    };

    //gives checkout session id for future reference
    let checkout_session_id = checkout_session.id.to_string();

    //gives checkout url
    let checkout_url = match checkout_session.url {
        Some(url) => url,
        None => {
            return HttpResponse::InternalServerError()
                .body("Stripe API error: no checkout URL returned");
        }
    };

    //save payment_id first because inserted_payment will be moved by into_active_model()
    let payment_id = inserted_payment.payment_id;

    //saving checkout session id to payment record in database
    let mut active_payment = inserted_payment.into_active_model();
    active_payment.checkout_session_id = Set(Some(checkout_session_id.clone()));

    //update checkout session id in db from NULL to session_id.
    match active_payment.update(db.get_ref()).await {
        Ok(_) => {
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




#[post("/stripe/webhook")]
pub async fn stripe_webhook(
    db: web::Data<DatabaseConnection>,
    req: HttpRequest,
    body: String,
) -> impl Responder {
    println!("Webhook endpoint hit");

    // Get the Stripe signature header from the request.
    let stripe_signature = match req.headers().get("stripe-signature") {
        Some(signature) => match signature.to_str() {
            Ok(value) => value,
            Err(_) => {
                return HttpResponse::BadRequest()
                    .body("Invalid stripe-signature header");
            }
        },
        None => {
            return HttpResponse::BadRequest()
                .body("Missing stripe-signature header");
        }
    };

    //get webhook secret key from .env
    let webhook_secret = match std::env::var("STRIPE_WEBHOOK_SECRET") {
        Ok(secret) => secret,
        Err(_) => {
            return HttpResponse::InternalServerError()
                .body("Stripe webhook secret missing in .env");
        }
    };

    //verification that webook request from stripe
    let event = match Webhook::construct_event(&body, stripe_signature, &webhook_secret) {
        Ok(event) => event,
        Err(err) => {
            return HttpResponse::BadRequest()
                .body(format!("Webhook verification failed: {}", err));
        }
    };

    println!("Webhook event type: {:?}", event.type_);

    //handle stripe event type
    match event.type_ {
        //handles checkout.session.completed event
        EventType::CheckoutSessionCompleted => {
            handle_checkout_session_completed(db, event).await
        }

        //handles payment_intent.succeeded event
        EventType::PaymentIntentSucceeded => {
            handle_payment_intent_succeeded(db, event).await
        }

        //ignore other events for now
        _ => {
            println!("Event ignored");
            HttpResponse::Ok().body("Event ignored")
        }
    }
}


//handles checkout.session.completed event
async fn handle_checkout_session_completed(
    db: web::Data<DatabaseConnection>,
    event: stripe::Event,
) -> HttpResponse {
    let session = match event.data.object {
        EventObject::CheckoutSession(session) => session,
        _ => {
            return HttpResponse::BadRequest()
                .body("Event object is not a CheckoutSession");
        }
    };

    println!("Checkout session received");
    println!("Session payment status: {:?}", session.payment_status);

    // Extra safety: only fulfill if Stripe says payment_status is paid.
    if session.payment_status != CheckoutSessionPaymentStatus::Paid {
        return HttpResponse::Ok()
            .body("Checkout session completed but payment is not paid");
    }

    //get metadata from stripe
    let metadata = match session.metadata {
        Some(metadata) => metadata,
        None => {
            return HttpResponse::BadRequest()
                .body("No metadata found in checkout session");
        }
    };

    let payment_id: i32 = match metadata.get("payment_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest()
                .body("Missing or invalid payment_id metadata");
        }
    };

    let user_id: i32 = match metadata.get("user_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest()
                .body("Missing or invalid user_id metadata");
        }
    };

    let course_id: i32 = match metadata.get("course_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest()
                .body("Missing or invalid course_id metadata");
        }
    };

    //get stripe payment reference id
    let payment_ref = session.payment_intent
        .map(|payment_intent| payment_intent.id().to_string());

    fulfill_payment(db, payment_id, user_id, course_id, payment_ref).await
}


//handles payment_intent.succeeded event
async fn handle_payment_intent_succeeded(
    db: web::Data<DatabaseConnection>,
    event: stripe::Event,
) -> HttpResponse {
    let payment_intent = match event.data.object {
        EventObject::PaymentIntent(payment_intent) => payment_intent,
        _ => {
            return HttpResponse::BadRequest()
                .body("Event object is not a PaymentIntent");
        }
    };

    println!("PaymentIntent succeeded received");

    //get metadata from stripe
    let metadata = payment_intent.metadata;

    let payment_id: i32 = match metadata.get("payment_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest()
                .body("Missing or invalid payment_id metadata");
        }
    };

    let user_id: i32 = match metadata.get("user_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest()
                .body("Missing or invalid user_id metadata");
        }
    };

    let course_id: i32 = match metadata.get("course_id").and_then(|v| v.parse::<i32>().ok()) {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest()
                .body("Missing or invalid course_id metadata");
        }
    };

    //get stripe payment reference id
    let payment_ref = Some(payment_intent.id.to_string());

    fulfill_payment(db, payment_id, user_id, course_id, payment_ref).await
}


//updates payment record and inserts enrollment after successful payment
async fn fulfill_payment(
    db: web::Data<DatabaseConnection>,
    payment_id: i32,
    user_id: i32,
    course_id: i32,
    payment_ref: Option<String>,
) -> HttpResponse {
    // Find existing payment row.
    let existing_payment = match payments::Entity::find_by_id(payment_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(payment)) => payment,
        Ok(None) => {
            return HttpResponse::NotFound()
                .body("Payment row not found");
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding payment: {}", err));
        }
    };

    //validation if webbhook sent twice
    if existing_payment.payment_status == "SUCCEEDED" {
        return HttpResponse::Ok()
            .body("Payment already fulfilled");
    }

    //update payment record in database to "SUCCEEDED" after successful payment
    let mut active_payment = existing_payment.into_active_model();
    active_payment.payment_status = Set("SUCCEEDED".to_string());
    active_payment.payment_ref = Set(payment_ref);
    active_payment.paid_at = Set(Some(Utc::now()));

    if let Err(err) = active_payment.update(db.get_ref()).await {
        return HttpResponse::InternalServerError()
            .body(format!("Database error updating payment: {}", err));
    }

    println!("Payment updated to SUCCEEDED");

    //insert into entollment
    let enrollment = crate::entity::enrollments::ActiveModel {
        user_id: Set(user_id),
        course_id: Set(course_id),
        ..Default::default()
    };

    match enrollment.insert(db.get_ref()).await {
        Ok(_) => {
            println!("Enrollment created");
            HttpResponse::Ok()
                .body("Payment succeeded and enrollment created")
        }
        Err(err) => {
            // If the user is already enrolled, this may fail because of primary key conflict.
            // That is acceptable for duplicate webhook deliveries.
            println!("Enrollment insert issue: {}", err);
            HttpResponse::Ok()
                .body(format!("Payment updated, enrollment may already exist: {}", err))
        }
    }
}