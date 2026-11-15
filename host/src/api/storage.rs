//! Owner-facing storage intents: the S3 destination is set on the computer, and a document's export
//! is uploaded with one call. Paired devices read `s3Status` only.

use super::str_arg;
use crate::hub::Hub;
use crate::s3::{self, Credentials, Settings};
use anyhow::{Context, Result};
use serde_json::Value;
use std::sync::Arc;

pub(super) async fn call(hub: &Arc<Hub>, method: &str, body: Value) -> Result<Value> {
    Ok(match method {
        "s3Status" => s3::status(&hub.store)?,
        "setS3" => {
            let text = |key: &str| body[key].as_str().unwrap_or_default().trim().to_owned();
            let settings = Settings {
                endpoint: text("endpoint"),
                region: text("region"),
                bucket: text("bucket"),
                prefix: text("prefix").trim_matches('/').to_owned(),
            };
            let credentials = Credentials {
                access_key_id: text("accessKeyId"),
                secret_access_key: body["secretAccessKey"].as_str().unwrap_or_default().trim().to_owned(),
            };
            s3::save(&hub.store, &settings, &credentials)?;
            s3::status(&hub.store)?
        }
        "disconnectS3" => {
            s3::disconnect(&hub.store)?;
            s3::status(&hub.store)?
        }
        _ => {
            let key = body["key"].as_str().map(str::to_owned);
            s3::upload_document(
                hub,
                str_arg(&body, "botId")?,
                str_arg(&body, "id")?,
                str_arg(&body, "format")?,
                key.as_deref(),
            )
            .await
            .context("the document was exported but not uploaded")?
        }
    })
}
