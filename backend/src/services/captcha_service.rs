use serde::Deserialize;
use std::env;

const RECAPTCHA_VERIFY_URL: &str = "https://www.google.com/recaptcha/api/siteverify";
const RECAPTCHA_TEST_SITE_KEY: &str = "6LeIxAcTAAAAAJcZVRqyHh71UMIEGNQ_MXjiZKhI";
const RECAPTCHA_TEST_SECRET_KEY: &str = "6LeIxAcTAAAAAGG-vFI1TnRWxMZNFuojJ4WifJWe";

#[derive(Debug, Deserialize)]
struct RecaptchaVerifyResponse {
    success: bool,
    #[serde(rename = "error-codes")]
    error_codes: Option<Vec<String>>,
}

pub fn recaptcha_site_key() -> Result<String, String> {
    let configured_key = env::var("RECAPTCHA_SITE_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty());

    configured_key
        .or_else(|| cfg!(debug_assertions).then(|| RECAPTCHA_TEST_SITE_KEY.to_string()))
        .ok_or_else(|| {
            "Google reCAPTCHA is not configured. Missing RECAPTCHA_SITE_KEY.".to_string()
        })
}

fn recaptcha_secret_key() -> Result<String, String> {
    let configured_key = env::var("RECAPTCHA_SECRET_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty());

    configured_key
        .or_else(|| cfg!(debug_assertions).then(|| RECAPTCHA_TEST_SECRET_KEY.to_string()))
        .ok_or_else(|| {
            "Google reCAPTCHA is not configured. Missing RECAPTCHA_SECRET_KEY.".to_string()
        })
}

pub async fn verify_recaptcha(
    response_token: Option<&str>,
    remote_ip: Option<String>,
) -> Result<bool, String> {
    let response_token = match response_token
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(token) => token.to_string(),
        None => return Ok(false),
    };

    let secret = recaptcha_secret_key()?;
    let mut params = vec![
        ("secret".to_string(), secret),
        ("response".to_string(), response_token),
    ];

    if let Some(remote_ip) = remote_ip.filter(|value| !value.trim().is_empty()) {
        params.push(("remoteip".to_string(), remote_ip));
    }

    let verification = reqwest::Client::new()
        .post(RECAPTCHA_VERIFY_URL)
        .form(&params)
        .send()
        .await
        .map_err(|err| {
            println!("reCAPTCHA verify request error: {:?}", err);
            "Unable to verify reCAPTCHA right now.".to_string()
        })?
        .json::<RecaptchaVerifyResponse>()
        .await
        .map_err(|err| {
            println!("reCAPTCHA verify response parse error: {:?}", err);
            "Unable to verify reCAPTCHA right now.".to_string()
        })?;

    if !verification.success {
        println!(
            "reCAPTCHA verification failed: {:?}",
            verification.error_codes
        );
    }

    Ok(verification.success)
}
