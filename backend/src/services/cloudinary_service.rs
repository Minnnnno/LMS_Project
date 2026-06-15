use reqwest::multipart;
use serde::{Deserialize, Serialize};

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

    let resource_type = "auto";

    let upload_url = format!(
        "https://api.cloudinary.com/v1_1/{}/{}/upload",
        cloud_name,
        resource_type
    );

    let file_part = multipart::Part::bytes(file_bytes).file_name(filename);
    let form = multipart::Form::new()
        .part("file", file_part)
        .text("folder", folder);

    let response = reqwest::Client::new()
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

    response
        .json::<CloudinaryUploadResponse>()
        .await
        .map_err(|err| err.to_string())
}
