use actix_web::cookie::Key;

pub fn is_production() -> bool {
    std::env::var("APP_ENV")
        .map(|value| value.eq_ignore_ascii_case("production"))
        .unwrap_or(false)
}

pub fn session_key() -> Key {
    match std::env::var("SESSION_SECRET") {
        Ok(secret) if secret.as_bytes().len() >= 64 => Key::derive_from(secret.as_bytes()),
        Ok(_) if is_production() => {
            panic!("SESSION_SECRET must contain at least 64 characters in production")
        }
        Err(_) if is_production() => {
            panic!("SESSION_SECRET must be set in production")
        }
        _ => {
            eprintln!(
                "WARNING: SESSION_SECRET is missing or too short. Sessions will reset when the development server restarts."
            );
            Key::generate()
        }
    }
}

pub fn cors_allowed_origin() -> String {
    let configured = std::env::var("CORS_ALLOWED_ORIGIN")
        .or_else(|_| std::env::var("FRONTEND_URL"))
        .unwrap_or_else(|_| {
            if is_production() {
                panic!("CORS_ALLOWED_ORIGIN or FRONTEND_URL must be set in production");
            }
            "http://127.0.0.1:8080".to_string()
        });

    let origin = configured.trim().trim_end_matches('/').to_string();
    let parsed = url::Url::parse(&origin)
        .unwrap_or_else(|_| panic!("CORS allowed origin is not a valid URL: {origin}"));

    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || (parsed.path() != "/" && !parsed.path().is_empty())
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        panic!("CORS allowed origin must contain only scheme, host, and optional port: {origin}");
    }

    origin
}
