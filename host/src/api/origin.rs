//! Browser origins are a separate boundary from bearer tokens and E2E identity.

use axum::Json;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header::ORIGIN};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

fn allowed(headers: &HeaderMap, port: u16) -> bool {
    let mut origins = headers.get_all(ORIGIN).iter();
    let Some(origin) = origins.next() else {
        // Native clients and server-to-server webhooks don't send Origin.
        return true;
    };
    if origins.next().is_some() {
        return false;
    }
    let Ok(origin) = origin.to_str() else { return false };
    ["127.0.0.1", "localhost", "[::1]"].iter().any(|host| origin == format!("http://{host}:{port}"))
}

pub(super) async fn guard(State(port): State<u16>, request: Request, next: Next) -> Response {
    if !allowed(request.headers(), port) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "untrusted browser origin"}))).into_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn native_clients_and_exact_local_origins_are_accepted() {
        let mut headers = HeaderMap::new();
        assert!(allowed(&headers, 19300));
        for origin in ["http://127.0.0.1:19300", "http://localhost:19300", "http://[::1]:19300"] {
            headers.insert(ORIGIN, origin.parse().unwrap());
            assert!(allowed(&headers, 19300), "{origin}");
        }
    }

    #[test]
    fn host_headers_do_not_authorize_foreign_opaque_or_ambiguous_origins() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "attacker.example".parse().unwrap());
        for origin in [
            "null",
            "https://attacker.example",
            "http://attacker.example",
            "http://localhost:19301",
            "https://localhost:19300",
            "http://localhost:19300.attacker.example",
            "http://localhost:19300/",
            "http://localhost:19300 http://attacker.example",
            "http://user@localhost:19300",
        ] {
            headers.insert(ORIGIN, origin.parse().unwrap());
            assert!(!allowed(&headers, 19300), "{origin}");
        }
        headers.insert(ORIGIN, "http://localhost:19300".parse().unwrap());
        headers.append(ORIGIN, "http://localhost:19300".parse().unwrap());
        assert!(!allowed(&headers, 19300));
        headers.remove(ORIGIN);
        headers.insert(ORIGIN, HeaderValue::from_bytes(b"\xff").unwrap());
        assert!(!allowed(&headers, 19300));
    }

    #[tokio::test]
    async fn foreign_origins_are_refused_before_the_route_executes() {
        use axum::{Router, middleware, routing::post};
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let router = Router::new()
            .route(
                "/api/action",
                post(move || async move {
                    observed.fetch_add(1, Ordering::SeqCst);
                    StatusCode::NO_CONTENT
                }),
            )
            .layer(middleware::from_fn_with_state(port, guard));
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{port}/api/action");
        let response = client.post(&url).header(ORIGIN, "https://attacker.example").send().await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(client.post(&url).send().await.unwrap().status(), StatusCode::NO_CONTENT);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        server.abort();
    }
}
