//! Local, offline mocks for agent tests: an OpenAI-compatible provider, an MCP echo server, an A2A agent and
//! an AG-UI endpoint, on one loopback port.
//!
//! They exist only in debug builds, behind the hidden `codync-host mock` subcommand, so no release binary
//! carries a test server. `host/tests/mock_offline_e2e.rs` starts it and runs one turn through it; the marker
//! each endpoint understands (`[mock:error]`, `[mock:tool]`, …) is documented in the test and in
//! `docs/guides/test-stack.md`.

mod a2a;
mod agui;
mod mcp;
mod provider;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;

/// `codync-host mock`: serve every mock endpoint on one loopback port.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Port to listen on; 0 (the default) picks a free one, printed as `{"port":N}`.
    #[arg(long, default_value_t = 0)]
    port: u16,
}

pub(crate) async fn serve(args: Args) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", args.port)).await?;
    let port = listener.local_addr()?.port();
    // The test helper reads this line to learn the port; stdout is line-buffered.
    println!("{}", serde_json::json!({"port": port}));
    axum::serve(listener, router()).await?;
    Ok(())
}

/// Every mock endpoint, here and in [`serve`].
pub(crate) fn router() -> Router {
    Router::new()
        .route("/health", get(|| async { (StatusCode::OK, "ok") }))
        .merge(provider::router())
        .merge(mcp::router())
        .merge(a2a::router())
        .merge(agui::router())
}
