use actix_web::{
    Error,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorForbidden,
    http::{Method, header},
    middleware::Next,
    web,
};

#[derive(Clone)]
pub struct CsrfConfig {
    allowed_origin: String,
}

impl CsrfConfig {
    pub fn new(allowed_origin: String) -> Self {
        Self { allowed_origin }
    }
}

const EXEMPT_PATHS: &[&str] = &["/api/stripe/webhook"];

fn is_safe_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    )
}

fn referer_origin(referer: &str) -> Option<String> {
    let parsed = url::Url::parse(referer).ok()?;
    Some(parsed.origin().ascii_serialization())
}

fn has_allowed_source(origin: Option<&str>, referer: Option<&str>, allowed_origin: &str) -> bool {
    origin
        .map(|value| value.trim_end_matches('/') == allowed_origin)
        .unwrap_or(false)
        || referer
            .and_then(referer_origin)
            .map(|value| value == allowed_origin)
            .unwrap_or(false)
}

pub async fn csrf_protection(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    if is_safe_method(req.method()) || EXEMPT_PATHS.contains(&req.path()) {
        return next.call(req).await;
    }

    let config = req
        .app_data::<web::Data<CsrfConfig>>()
        .ok_or_else(|| ErrorForbidden("CSRF protection is not configured"))?;
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    let referer = req
        .headers()
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok());

    if !has_allowed_source(origin, referer, &config.allowed_origin) {
        return Err(ErrorForbidden("Cross-site request rejected"));
    }

    next.call(req).await
}

#[cfg(test)]
mod tests {
    use super::has_allowed_source;

    const ALLOWED: &str = "https://lms.example.com";

    #[test]
    fn accepts_matching_origin_or_referer() {
        assert!(has_allowed_source(Some(ALLOWED), None, ALLOWED));
        assert!(has_allowed_source(
            None,
            Some("https://lms.example.com/courses/12"),
            ALLOWED
        ));
    }

    #[test]
    fn rejects_cross_site_or_missing_source() {
        assert!(!has_allowed_source(
            Some("https://attacker.example"),
            None,
            ALLOWED
        ));
        assert!(!has_allowed_source(None, None, ALLOWED));
    }
}
