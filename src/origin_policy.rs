use axum::{
    extract::Request,
    http::{header::ORIGIN, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

fn is_allowed_origin(origin: &HeaderValue) -> bool {
    let Ok(value) = origin.to_str() else {
        return false;
    };
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };

    matches!(url.scheme(), "http" | "https")
        && matches!(
            url.host_str(),
            Some("tailrmade.app" | "plugandplayground.dev" | "localhost" | "127.0.0.1")
        )
        && url.origin().ascii_serialization() == value
}

pub fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            is_allowed_origin(origin)
        }))
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any)
}

pub async fn check_origin(request: Request, next: Next) -> Result<Response, StatusCode> {
    // Navigations and non-browser clients may omit Origin.
    let mut origins = request.headers().get_all(ORIGIN).iter();
    if let Some(origin) = origins.next() {
        if !is_allowed_origin(origin) || origins.next().is_some() {
            tracing::warn!(
                ?origin,
                path = request.uri().path(),
                "Rejected request from disallowed origin"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_exact_domains_with_http_https_and_ports() {
        for domain in [
            "tailrmade.app",
            "plugandplayground.dev",
            "localhost",
            "127.0.0.1",
        ] {
            for scheme in ["http", "https"] {
                for port in ["", ":3000", ":6655"] {
                    let origin = format!("{scheme}://{domain}{port}");
                    assert!(is_allowed_origin(&origin.parse().unwrap()), "{origin}");
                }
            }
        }
    }

    #[test]
    fn rejects_other_domains_and_invalid_origins() {
        for origin in [
            "https://example.com",
            "https://www.tailrmade.app",
            "https://tailrmade.app.evil.com",
            "https://eviltailrmade.app",
            "https://localhost.evil.com",
            "http://127.0.0.2:6655",
            "http://127.0.0.1.evil.com:6655",
            "http://[::1]:6655",
            "https://tailrmade.app@evil.com",
            "https://evil.com@tailrmade.app",
            "https://tailrmade.app/path",
            "https://tailrmade.app?query",
            "https://tailrmade.app#fragment",
            "https://tailrmade.app https://localhost",
            "ftp://tailrmade.app",
            "null",
            "localhost",
            "",
        ] {
            assert!(!is_allowed_origin(&origin.parse().unwrap()), "{origin}");
        }
    }
}
