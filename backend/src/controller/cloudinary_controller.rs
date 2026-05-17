use actix_web::{HttpResponse, Responder};
use chrono::Utc;
use serde::Serialize;
use sha1::{Digest, Sha1};

#[derive(Serialize)]
struct CloudinarySignature {
    timestamp: i64,
    signature: String,
    api_key: String,
    cloud_name: String,
}

pub async fn get_upload_signature() -> impl Responder {
    let cloud_name: String = std::env::var("CLOUDINARY_CLOUD_NAME")
    .expect("CLOUDINARY_CLOUD_NAME not found");
    let api_key: String = std::env::var("CLOUDINARY_API_KEY")
        .expect("CLOUDINARY_API_KEY not found");

    let api_secret: String = std::env::var("CLOUDINARY_API_SECRET")
        .expect("CLOUDINARY_API_SECRET not found");
    let timestamp = Utc::now().timestamp();

    let string_to_sign = format!("timestamp={}{}", timestamp, api_secret);

    let mut hasher = Sha1::new();
    hasher.update(string_to_sign.as_bytes());

    let signature = hex::encode(hasher.finalize());

    HttpResponse::Ok().json(CloudinarySignature {
        timestamp,
        signature,
        api_key,
        cloud_name,
    })
}