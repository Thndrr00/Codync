//! Optional embeddings from a local Ollama (`/api/embeddings`), used to rerank workspace search
//! results. Nothing is required: without Ollama the caller keeps the lexical order. The address and
//! model are read from `CODYNC_OLLAMA_URL` and `CODYNC_EMBED_MODEL`, so a person can point it at
//! their own install without a setting screen.

use std::time::Duration;

const DEFAULT_URL: &str = "http://127.0.0.1:11434";
const DEFAULT_MODEL: &str = "nomic-embed-text";

fn endpoint() -> String {
    std::env::var("CODYNC_OLLAMA_URL").unwrap_or_else(|_| DEFAULT_URL.into()).trim_end_matches('/').to_owned()
}

fn model() -> String {
    std::env::var("CODYNC_EMBED_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.into())
}

/// One text's embedding, or `None` when Ollama is not running, has no such model or answers badly.
pub async fn embed(text: &str) -> Option<Vec<f32>> {
    let response = crate::http()
        .post(format!("{}/api/embeddings", endpoint()))
        .json(&serde_json::json!({"model": model(), "prompt": text}))
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let value: serde_json::Value = response.json().await.ok()?;
    let embedding: Vec<f32> = value["embedding"]
        .as_array()?
        .iter()
        .filter_map(serde_json::Value::as_f64)
        .map(|number| number as f32)
        .collect();
    (!embedding.is_empty()).then_some(embedding)
}

/// Cosine similarity in [-1, 1]; 0 when either vector is empty or they differ in length.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let (mut dot, mut aa, mut bb) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        aa += x * x;
        bb += y * y;
    }
    if aa == 0.0 || bb == 0.0 { 0.0 } else { dot / (aa.sqrt() * bb.sqrt()) }
}
