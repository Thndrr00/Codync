//! The remote AG-UI endpoint a bot points at: where its runs are posted and the optional bearer token.
//!
//! HTTPS anywhere; HTTP only on this computer (the same rule as the local voice server). The values reach the
//! host-side ACP adapter as environment and live in the vault like every other agent key.

use anyhow::{Result, anyhow, bail};

/// The endpoint URL, saved from the agent's settings (`setAgentEnv`).
pub const URL_ENV: &str = "CODYNC_AGUI_URL";
/// The optional bearer token the endpoint takes.
pub const TOKEN_ENV: &str = "CODYNC_AGUI_TOKEN";

/// The endpoint and token from this process's environment.
pub fn from_env() -> Result<(reqwest::Url, Option<String>)> {
    let value = std::env::var(URL_ENV).unwrap_or_default();
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow!(
            "This bot's AG-UI agent has no endpoint. Open Extensions > AG-UI agent and save the endpoint URL."
        ));
    }
    let endpoint = parse_endpoint(value, "The AG-UI endpoint URL")?;
    let token = std::env::var(TOKEN_ENV).unwrap_or_default();
    let token = token.trim();
    Ok((endpoint, (!token.is_empty()).then(|| token.to_owned())))
}

/// An endpoint as saved: HTTPS, or HTTP only on this computer, and no credentials, query or fragment.
pub fn parse_endpoint(value: &str, label: &str) -> Result<reqwest::Url> {
    let value = value.trim();
    let mut url = reqwest::Url::parse(value)
        .map_err(|_| anyhow!("{label} must be an absolute HTTP(S) URL, such as https://agent.example/agui"))?;
    let userinfo = value
        .split_once("://")
        .is_some_and(|(_, rest)| rest.split(['/', '?', '#']).next().is_some_and(|authority| authority.contains('@')));
    if userinfo
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("{label} must not contain credentials, a query or a fragment");
    }
    let loopback = url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host.trim_matches(['[', ']']).parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
    });
    if url.host_str().is_none() || !(url.scheme() == "https" || (url.scheme() == "http" && loopback)) {
        bail!("{label} requires HTTPS, except HTTP on localhost or a loopback IP");
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_anywhere_and_http_on_this_computer_are_accepted() {
        for endpoint in [
            "https://agent.example/agui",
            "https://agent.example:8443/run/",
            "http://localhost:8000/agui",
            "http://127.0.0.2:8000",
            "http://[::1]:8000/",
        ] {
            assert!(parse_endpoint(endpoint, "The endpoint").is_ok(), "{endpoint}");
        }
    }

    #[test]
    fn insecure_or_ambiguous_endpoints_are_refused() {
        for endpoint in [
            "http://agent.example/agui",
            "http://192.168.1.10:8000",
            "http://localhost.example",
            "ftp://localhost",
            "/relative",
            "https://user:secret@agent.example",
            "https://agent.example?key=secret",
            "https://agent.example/#fragment",
        ] {
            assert!(parse_endpoint(endpoint, "The endpoint").is_err(), "accepted {endpoint}");
        }
    }
}
