use actix_multipart::Multipart;
use actix_web::{post, HttpResponse, Responder};
use futures_util::StreamExt;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use crate::services::malware_scanner::scan_file_for_malware;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudinaryUploadResponse {
    pub secure_url: String,
    pub public_id: String,
}

pub async fn upload_to_cloudinary(
    file_bytes: Vec<u8>,
    filename: String,
    folder: String,
) -> Result<CloudinaryUploadResponse, String> {
    let cloud_name = std::env::var("CLOUDINARY_CLOUD_NAME")
        .map_err(|_| "CLOUDINARY_CLOUD_NAME not found".to_string())?;

    let api_key = std::env::var("CLOUDINARY_API_KEY")
        .map_err(|_| "CLOUDINARY_API_KEY not found".to_string())?;

    let api_secret = std::env::var("CLOUDINARY_API_SECRET")
        .map_err(|_| "CLOUDINARY_API_SECRET not found".to_string())?;

    let resource_type = if filename.to_lowercase().ends_with(".pdf") {
    "raw"
    } else {
        "image"
    };

    let upload_url = format!(
        "https://api.cloudinary.com/v1_1/{}/{}/upload",
        cloud_name,
        resource_type
    );

    if let Err(err) = scan_file_for_malware(&file_bytes).await {
        return Err(err);
    }
    let file_part = multipart::Part::bytes(file_bytes)
        .file_name(filename);

    let form = multipart::Form::new()
        .part("file", file_part)
        .text("folder", folder);
    let client = reqwest::Client::new();

    let response = client
        .post(upload_url)
        .basic_auth(api_key, Some(api_secret))
        .multipart(form)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(error_text);
    }

    let upload_result = response
        .json::<CloudinaryUploadResponse>()
        .await
        .map_err(|err| err.to_string())?;

    Ok(upload_result)
}

#[post("/cloudinary/upload")]
pub async fn upload_file(mut payload: Multipart) -> impl Responder {
    let mut file_bytes: Vec<u8> = Vec::new();
    let mut filename = String::from("upload_file");
    let mut folder = String::from("lms/uploads");

    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(field) => field,
            Err(err) => {
                return HttpResponse::BadRequest()
                    .body(format!("Multipart error: {}", err));
            }
        };

        let field_name = field
            .content_disposition()
            .and_then(|cd| cd.get_name())
            .map(|name| name.to_string());

        let field_filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename())
            .map(|name| name.to_string());

        if field_name.as_deref() == Some("file") {
            if let Some(file_name) = field_filename {
                filename = file_name;
            }

            while let Some(chunk) = field.next().await {
                match chunk {
                    Ok(data) => file_bytes.extend_from_slice(&data),
                    Err(err) => {
                        return HttpResponse::BadRequest()
                            .body(format!("File read error: {}", err));
                    }
                }
            }
        }

        if field_name.as_deref() == Some("folder") {
            let mut folder_bytes: Vec<u8> = Vec::new();

            while let Some(chunk) = field.next().await {
                match chunk {
                    Ok(data) => folder_bytes.extend_from_slice(&data),
                    Err(err) => {
                        return HttpResponse::BadRequest()
                            .body(format!("Folder read error: {}", err));
                    }
                }
            }

            folder = String::from_utf8(folder_bytes)
                .unwrap_or(String::from("lms/uploads"));
        }
    }

    if file_bytes.is_empty() {
        return HttpResponse::BadRequest().body("No file uploaded");
    }

    match upload_to_cloudinary(file_bytes, filename, folder).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Cloudinary upload error: {}", err)),
    }
}