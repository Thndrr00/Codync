//! The built-in bridge approves the provider tool, not the generic `execute` wrapper.
use super::{cancellation::Cancellation, policy};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::time::Duration;

pub(super) async fn execute(
    port: u16,
    token: &str,
    bot: &str,
    args: &Value,
    rpc_id: &Value,
    cancelled: &Cancellation,
) -> Result<Value> {
    if cancelled.cancelled(rpc_id) {
        bail!("Caller cancelled this request.");
    }
    let prepared =
        policy::proxy_call(port, token, "composioPrepareCall", &json!({"botId":bot,"arguments":args,"rpcId":rpc_id}))
            .await?;
    let connector = prepared["connectorId"].as_str().context("Missing Composio connector.")?;
    let fingerprint = prepared["fingerprint"].as_str().context("Missing Composio identity.")?;
    let (_, id) =
        policy::authorize(port, token, connector, prepared["message"].clone(), cancelled, fingerprint).await?;
    let id = id.context("Missing Composio authorization.")?;
    let body = json!({"id":id,"fingerprint":fingerprint});
    let dispatch = policy::proxy_call(port, token, "composioDispatchCall", &body);
    let monitor = async {
        loop {
            if cancelled.cancelled(rpc_id) {
                policy::proxy_call(port, token, "cancelMcpCall", &json!({"id":id})).await?;
                bail!(
                    "Composio request cancelled. If already dispatched, read its receipt before taking further action."
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        #[allow(unreachable_code)]
        anyhow::Ok(())
    };
    let reply = tokio::select! {reply=dispatch=>reply.with_context(||format!("Execution receipt {id}; outcome may be unknown. Retrieve the receipt without repeating the tool."))?, result=monitor=>{result?;bail!("Cancelled");}};
    if reply.get("error").is_some() {
        bail!("{}", reply["error"]["message"].as_str().unwrap_or("Composio tool failed"));
    }
    Ok(json!({"content":[{"type":"text","text":reply["result"].to_string()}],"_meta":{"codyncReceiptId":id}}))
}
