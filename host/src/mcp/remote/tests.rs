use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn tool_transport_never_follows_a_redirect_with_the_approved_arguments() {
    use axum::{Router, routing::post};
    let count = Arc::new(AtomicUsize::new(0));
    let target = count.clone();
    let router = Router::new()
        .route(
            "/redirect",
            post(|| async {
                (axum::http::StatusCode::TEMPORARY_REDIRECT, [(axum::http::header::LOCATION, "/target")])
            }),
        )
        .route(
            "/target",
            post(move || {
                let target = target.clone();
                async move {
                    target.fetch_add(1, Ordering::SeqCst);
                    "unexpected"
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let reply = client()
        .post(format!("http://{address}/redirect"))
        .json(&json!({"secret":"approved arguments"}))
        .send()
        .await
        .unwrap();
    assert_eq!(reply.status().as_u16(), 307);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    server.abort();
}
