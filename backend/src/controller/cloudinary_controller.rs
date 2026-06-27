use crate::services::cloudinary_service::upload_to_cloudinary;
use actix_multipart::Multipart;
use actix_web::{HttpResponse, Responder, post};
use futures_util::StreamExt;

#[post("/cloudinary/upload")]
pub async fn upload_file(mut payload: Multipart) -> impl Responder {
    let mut file_bytes: Vec<u8> = Vec::new();
    let mut filename = String::from("upload_file");
    let mut folder = String::from("lms/uploads");

    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(field) => field,
            Err(err) => {
                return HttpResponse::BadRequest().body(format!("Multipart error: {}", err));
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

            folder = String::from_utf8(folder_bytes).unwrap_or(String::from("lms/uploads"));
        }
    }

    if file_bytes.is_empty() {
        return HttpResponse::BadRequest().body("No file uploaded");
    }

    match upload_to_cloudinary(file_bytes, filename, folder).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Cloudinary upload error: {}", err))
        }
    }
}
