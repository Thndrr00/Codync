//! What a local model server offers, if one is running: `GET /api/tags` on Ollama. Semantic workspace
//! search uses its embeddings when present; the chat itself still runs through an agent harness.

use anyhow::Result;
use serde_json::{Value, json};
use std::time::Duration;

const DEFAULT_URL: &str = "http://127.0.0.1:11434";

fn endpoint() -> String {
    std::env::var("CODYNC_OLLAMA_URL").unwrap_or_else(|_| DEFAULT_URL.into()).trim_end_matches('/').to_owned()
}

/// `available` when `{url}/api/tags` answers, with the model names it lists.
pub async fn status() -> Result<Value> {
    let url = endpoint();
    let response = crate::http().get(format!("{url}/api/tags")).timeout(Duration::from_secs(3)).send().await;
    let Ok(response) = response else {
        return Ok(json!({"available": false, "url": url, "models": []}));
    };
    if !response.status().is_success() {
        return Ok(json!({"available": false, "url": url, "models": []}));
    }
    let value: Value = response.json().await.unwrap_or(Value::Null);
    let models: Vec<String> = value["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|model| model["name"].as_str().or_else(|| model["model"].as_str()).map(str::to_owned))
        .collect();
    Ok(json!({"available": true, "url": url, "models": models, "embeddingModel": embedding_model()}))
}

fn embedding_model() -> String {
    std::env::var("CODYNC_EMBED_MODEL").unwrap_or_else(|_| "nomic-embed-text".into())
}
