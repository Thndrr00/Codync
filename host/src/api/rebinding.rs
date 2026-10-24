//! DNS-rebinding guard. A web page can point its own domain at 127.0.0.1 and make the
//! user's browser call this host from that page; the browser then sends that domain in
//! `Host`. Requests from this computer must therefore name a loopback host. Requests from
//! other devices (phones on the LAN or Tailscale) name this computer's address and
//! authenticate through the E2E handshake, so they are left alone.

use axum::extract::{ConnectInfo, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::net::SocketAddr;

/// Rejects a loopback request whose `Host` isn't a loopback name.
pub async fn guard(ConnectInfo(peer): ConnectInfo<SocketAddr>, req: Request, next: Next) -> Response {
    let host = req.headers().get(axum::http::header::HOST).map(|h| h.to_str().unwrap_or_default());
    if peer.ip().to_canonical().is_loopback() && host.is_some_and(|h| !is_loopback_name(h)) {
        tracing::warn!(host = host.unwrap_or_default(), "refused a loopback request naming another host");
        return (StatusCode::FORBIDDEN, "host not allowed").into_response();
    }
    next.run(req).await
}

/// `localhost`, `127.0.0.1` or `[::1]`, with or without a port.
fn is_loopback_name(host: &str) -> bool {
    let name = match host.strip_prefix('[') {
        Some(rest) => rest.split_once(']').map_or("", |(name, _)| name),
        None => host.rsplit_once(':').map_or(host, |(name, _)| name),
    };
    let name = name.trim_end_matches('.').to_ascii_lowercase();
    name == "localhost" || name.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.to_canonical().is_loopback())
}

#[cfg(test)]
mod tests {
    use super::is_loopback_name;

    #[test]
    fn loopback_names_pass_with_or_without_a_port() {
        for host in ["localhost", "LOCALHOST:19222", "127.0.0.1", "127.0.0.1:19222", "[::1]", "[::1]:19222"] {
            assert!(is_loopback_name(host), "{host} names this computer");
        }
    }

    #[test]
    fn rebinding_domains_and_other_addresses_are_refused() {
        for host in ["evil.example", "evil.example:19222", "localhost.evil.example", "192.168.1.20:19222", "", "[::1"] {
            assert!(!is_loopback_name(host), "{host} is not a loopback name");
        }
    }
}
