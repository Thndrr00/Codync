//! OpenAI-compatible speech endpoint validation and credential-safe requests.

use anyhow::{Result, anyhow, bail};

/// `CODYNC_OPENAI_BASE_URL` (e.g. `http://127.0.0.1:8890`) points the `OpenAI` engine at a
/// compatible server on this computer instead of `api.openai.com`: local speech to text and
/// text to speech. Unset, nothing changes.
pub(super) fn openai_base() -> Result<Option<reqwest::Url>> {
    parse_openai_base(std::env::var("CODYNC_OPENAI_BASE_URL").ok().as_deref())
}

fn parse_openai_base(value: Option<&str>) -> Result<Option<reqwest::Url>> {
    let Some(value) = value.map(str::trim).filter(|s| !s.is_empty()) else { return Ok(None) };
    let mut url =
        reqwest::Url::parse(value).map_err(|_| anyhow!("CODYNC_OPENAI_BASE_URL must be an absolute HTTP(S) URL"))?;
    let userinfo = value
        .split_once("://")
        .is_some_and(|(_, rest)| rest.split(['/', '?', '#']).next().is_some_and(|a| a.contains('@')));
    if userinfo
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("CODYNC_OPENAI_BASE_URL must not contain credentials, a query or a fragment");
    }
    let loopback = url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host.trim_matches(['[', ']']).parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
    });
    if url.host_str().is_none() || !(url.scheme() == "https" || (url.scheme() == "http" && loopback)) {
        bail!("CODYNC_OPENAI_BASE_URL requires HTTPS, except HTTP on localhost or a loopback IP");
    }
    let path = url.path().trim_end_matches('/');
    let path = path.strip_suffix("/v1").unwrap_or(path).to_owned();
    url.set_path(&path);
    Ok(Some(url))
}

pub(super) fn openai_url(base: Option<&reqwest::Url>, path: &str) -> String {
    format!("{}{path}", base.map_or("https://api.openai.com", |url| url.as_str().trim_end_matches('/')))
}

pub(super) fn realtime_url(base: Option<&reqwest::Url>) -> Result<String> {
    if base.is_some() {
        bail!(
            "OpenAI Realtime is unavailable with CODYNC_OPENAI_BASE_URL. Use Speech mode or remove the custom endpoint."
        );
    }
    Ok(openai_url(None, "/v1/realtime/client_secrets"))
}

// Never forward a saved bearer key through a custom server's redirect (even on the same host).
pub(super) fn openai_http() -> Result<&'static reqwest::Client> {
    static CLIENT: std::sync::LazyLock<std::result::Result<reqwest::Client, reqwest::Error>> =
        std::sync::LazyLock::new(|| reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build());
    CLIENT.as_ref().map_err(|_| anyhow!("Could not initialize the OpenAI speech client"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn compatible_base_normalizes_version_and_preserves_proxy_prefix() {
        for base in
            ["https://speech.example/proxy", "https://speech.example/proxy/", "https://speech.example/proxy/v1/"]
        {
            let base = parse_openai_base(Some(base)).unwrap();
            assert_eq!(openai_url(base.as_ref(), "/v1/audio/speech"), "https://speech.example/proxy/v1/audio/speech");
        }
        for base in ["http://localhost:8890/v1", "http://127.0.0.2:8890", "http://[::1]:8890/"] {
            assert!(parse_openai_base(Some(base)).is_ok());
        }
        assert!(parse_openai_base(Some("  ")).unwrap().is_none());
        assert_eq!(openai_url(None, "/v1/models"), "https://api.openai.com/v1/models");
    }

    #[test]
    fn compatible_base_rejects_insecure_or_ambiguous_destinations() {
        for base in [
            "http://speech.example",
            "http://192.168.1.10:8890",
            "http://localhost.example",
            "ftp://localhost",
            "/relative",
            "https://user:secret@speech.example",
            "https://@speech.example",
            "https://speech.example?key=secret",
            "https://speech.example/#fragment",
        ] {
            assert!(parse_openai_base(Some(base)).is_err(), "accepted {base}");
        }
    }

    #[test]
    fn compatible_speech_endpoint_cannot_mint_realtime_credentials() {
        let base = parse_openai_base(Some("http://127.0.0.1:8890/v1")).unwrap();
        assert!(realtime_url(base.as_ref()).is_err());
        assert_eq!(realtime_url(None).unwrap(), "https://api.openai.com/v1/realtime/client_secrets");
    }

    #[tokio::test]
    async fn speech_client_does_not_follow_credential_redirects() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 2048];
            let _ = stream.read(&mut buffer).await.unwrap();
            stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/collect\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        });
        let response = openai_http()
            .unwrap()
            .get(format!("http://{address}/v1/models"))
            .bearer_auth("test-only-key")
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        server.await.unwrap();
    }
}
