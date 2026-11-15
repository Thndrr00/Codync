//! Uploading a file to `S3`-compatible storage (`AWS`, `Cloudflare R2`, `MinIO`) with AWS Signature
//! Version 4.
//!
//! The settings (endpoint, region, bucket, prefix) are ordinary key-value records; the access key
//! and the secret live in the vault. Nothing here runs on its own: a person configures the
//! destination on the computer, then asks for one upload, which signs that one request.
//!
//! Path-style requests are the only shape: `\<endpoint\>/\<bucket\>/\<key\>`, which AWS still serves and
//! every compatible server understands. The signed headers are the three that must not change on
//! the way (`host`, `x-amz-content-sha256`, `x-amz-date`); the payload hash covers the body.
//!
//! An upload names its object key or lets the configured prefix and the file's own name build one.

use crate::hub::Hub;
use crate::store::Store;
use anyhow::{Context, Result, bail, ensure};
use chrono::DateTime;
use hmac::{Hmac, KeyInit as _, Mac as _};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

pub const METHODS: &[&str] = &["s3Status", "setS3", "disconnectS3", "uploadDocument"];

const SETTINGS: &str = "s3.settings";
const CREDENTIALS: &str = "s3.credentials";
/// Covers the body with a digest, which S3 requires of every signed request.
const PAYLOAD_HEADER: &str = "x-amz-content-sha256";
/// Beyond this many bytes the failure body isn't repeated back.
const MAX_ERROR_BODY: usize = 500;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    /// `https://s3.us-east-1.amazonaws.com`, or any compatible server's base URL.
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    /// Folder the uploads go under, without leading or trailing slash.
    #[serde(default)]
    pub prefix: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
}

/// `s3Status`: what is configured, never the secret.
pub fn status(store: &Store) -> Result<Value> {
    let settings = load(store)?;
    Ok(json!({
        "configured": settings.is_some(),
        "endpoint": settings.as_ref().map(|s| s.endpoint.clone()),
        "region": settings.as_ref().map(|s| s.region.clone()),
        "bucket": settings.as_ref().map(|s| s.bucket.clone()),
        "prefix": settings.as_ref().map(|s| s.prefix.clone()).unwrap_or_default(),
        "hasCredentials": store.kv_read(CREDENTIALS)?.is_some(),
    }))
}

pub fn save(store: &Store, settings: &Settings, credentials: &Credentials) -> Result<()> {
    validate(settings)?;
    ensure!(!credentials.access_key_id.trim().is_empty(), "the access key id is empty");
    ensure!(!credentials.secret_access_key.trim().is_empty(), "the secret access key is empty");
    store.kv_set(SETTINGS, &serde_json::to_string(settings)?)?;
    crate::market::vault::write(store, CREDENTIALS, &serde_json::to_string(credentials)?)
}

pub fn disconnect(store: &Store) -> Result<()> {
    store.kv_delete(SETTINGS)?;
    Ok(())
}

fn load(store: &Store) -> Result<Option<Settings>> {
    store
        .kv_read(SETTINGS)?
        .map(|raw| serde_json::from_str(&raw).context("stored S3 settings are unreadable"))
        .transpose()
}

fn credentials(store: &Store) -> Result<Option<Credentials>> {
    crate::market::vault::read(store, CREDENTIALS)?
        .map(|raw| serde_json::from_str(&raw).context("stored S3 credentials are unreadable"))
        .transpose()
}

/// Settings and credentials, or the sentence saying what is missing.
fn configured(store: &Store) -> Result<(Settings, Credentials)> {
    let settings = load(store)?.context("S3 isn't set up: add an endpoint, region, bucket and access key")?;
    let credentials = credentials(store)?.context("S3 has no access key: add one")?;
    Ok((settings, credentials))
}

fn validate(settings: &Settings) -> Result<()> {
    let endpoint = url::Url::parse(settings.endpoint.trim()).context("the endpoint isn't a URL")?;
    ensure!(matches!(endpoint.scheme(), "http" | "https"), "the endpoint must be http or https");
    ensure!(endpoint.host_str().is_some(), "the endpoint has no host");
    ensure!(endpoint.query().is_none_or(str::is_empty), "the endpoint can't carry a query");
    ensure!(
        (3..=63).contains(&settings.bucket.len())
            && settings.bucket.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.')),
        "a bucket name is 3-63 lowercase letters, digits, - or ."
    );
    ensure!(!settings.region.trim().is_empty(), "the region is empty");
    ensure!(!settings.region.contains('/'), "the region can't contain /");
    ensure!(
        !settings.prefix.starts_with('/') && !settings.prefix.ends_with('/'),
        "the prefix has no leading or trailing /"
    );
    Ok(())
}

/// One signed request: the URL to call and the headers that carry the signature.
struct Signed {
    url: url::Url,
    headers: Vec<(String, String)>,
}

/// Parts of a request the signature covers, with the clock passed in so a test can fix it.
struct ToSign<'a> {
    method: &'a str,
    host: &'a str,
    /// Already URI-encoded path, each segment escaped, the slashes kept.
    path: &'a str,
    query: &'a str,
    headers: &'a [(&'a str, &'a str)],
    payload_hash: &'a str,
    region: &'a str,
    access_key: &'a str,
    secret_key: &'a str,
    at_ms: i64,
}

impl ToSign<'_> {
    /// `AWS4-HMAC-SHA256 Credential=…,SignedHeaders=…,Signature=…`, ready as `authorization`.
    fn authorization(&self) -> Result<String> {
        let at = DateTime::from_timestamp_millis(self.at_ms).context("the clock is out of range")?;
        let amz_date = at.format("%Y%m%dT%H%M%SZ").to_string();
        let day = at.format("%Y%m%d").to_string();
        let mut headers = vec![
            ("host".to_owned(), self.host.to_owned()),
            (PAYLOAD_HEADER.to_owned(), self.payload_hash.to_owned()),
            ("x-amz-date".to_owned(), amz_date.clone()),
        ];
        headers.extend(self.headers.iter().map(|(name, value)| (name.to_lowercase(), value.trim().to_owned())));
        headers.sort_by(|a, b| a.0.cmp(&b.0));
        let mut canonical_headers = String::new();
        for (name, value) in &headers {
            let _ = writeln!(canonical_headers, "{name}:{value}");
        }
        let signed_headers: Vec<&str> = headers.iter().map(|(name, _)| name.as_str()).collect();
        let signed = signed_headers.join(";");
        let canonical = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            self.method, self.path, self.query, canonical_headers, signed, self.payload_hash
        );
        let scope = format!("{day}/{}/s3/aws4_request", self.region);
        let to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}", sha256_hex(canonical.as_bytes()));
        let mut key = hmac(format!("AWS4{}", self.secret_key).as_bytes(), day.as_bytes())?;
        key = hmac(&key, self.region.as_bytes())?;
        key = hmac(&key, b"s3")?;
        key = hmac(&key, b"aws4_request")?;
        let signature = hex(&hmac(&key, to_sign.as_bytes())?);
        Ok(format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope},SignedHeaders={signed},Signature={signature}",
            self.access_key
        ))
    }
}

/// Signs one `PUT` of `key` with `body`: what the caller sends, with the headers that must go with
/// it.
fn sign_put(settings: &Settings, credentials: &Credentials, key: &str, body: &[u8], at_ms: i64) -> Result<Signed> {
    let mut url = base(settings)?;
    let path = format!("/{}/{}", settings.bucket, encode(key, true));
    url.set_path(&path);
    let host = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().context("the endpoint has no host")?),
        None => url.host_str().context("the endpoint has no host")?.to_owned(),
    };
    let payload_hash = sha256_hex(body);
    let headers = [("content-type", "application/octet-stream")];
    let authorization = ToSign {
        method: "PUT",
        host: &host,
        path: &path,
        query: "",
        headers: &headers,
        payload_hash: &payload_hash,
        region: &settings.region,
        access_key: &credentials.access_key_id,
        secret_key: &credentials.secret_access_key,
        at_ms,
    }
    .authorization()?;
    Ok(Signed {
        url,
        headers: vec![
            ("authorization".into(), authorization),
            ("x-amz-date".into(), utc_stamp(at_ms)?),
            (PAYLOAD_HEADER.into(), payload_hash),
            ("content-type".into(), headers[0].1.into()),
        ],
    })
}

/// The base URL: no trailing slash, so `Url::set_path` builds the whole path.
fn base(settings: &Settings) -> Result<url::Url> {
    let mut url = url::Url::parse(settings.endpoint.trim()).context("the endpoint isn't a URL")?;
    url.set_query(None);
    url.set_path("/");
    Ok(url)
}

/// The object's URL from the settings and key, as a person would see it.
fn object_url(settings: &Settings, key: &str) -> Result<url::Url> {
    let mut url = base(settings)?;
    url.set_path(&format!("/{}/{}", settings.bucket, encode(key, true)));
    Ok(url)
}

fn utc_stamp(at_ms: i64) -> Result<String> {
    let at = DateTime::from_timestamp_millis(at_ms).context("the clock is out of range")?;
    Ok(at.format("%Y%m%dT%H%M%SZ").to_string())
}

/// The upload's key: the configured prefix, then the file name, both URL-safe.
fn key_for(settings: &Settings, name: &str) -> String {
    let name: String =
        name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '-' }).collect();
    match settings.prefix.trim_matches('/') {
        "" => name,
        prefix => format!("{prefix}/{name}"),
    }
}

/// URI-encodes for the canonical path: every byte that isn't unreserved, except `/` when the text
/// is a whole key.
fn encode(text: &str, keep_slash: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            b'/' if keep_slash => out.push('/'),
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hmac(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).map_err(|_| anyhow::anyhow!("bad HMAC key"))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// Sends the signed request and turns a refusal into the server's own words.
async fn send(signed: Signed, body: Vec<u8>) -> Result<()> {
    let mut request = reqwest::Client::new().put(signed.url.clone()).body(body);
    for (name, value) in signed.headers {
        request = request.header(name, value);
    }
    let response = request.send().await.context("the storage server couldn't be reached")?;
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let mut text = response.text().await.unwrap_or_default();
    text.truncate(MAX_ERROR_BODY);
    bail!("the storage server refused the upload ({status}): {text}")
}

/// Exports a document and puts the copy in the bucket.
pub async fn upload_document(hub: &Hub, bot: &str, id: &str, format: &str, key: Option<&str>) -> Result<Value> {
    let (settings, credentials) = configured(&hub.store)?;
    let exported = crate::document_export::export(hub, bot, id, format)?;
    let path = exported["path"].as_str().context("the export has no path")?.to_owned();
    let body = std::fs::read(&path).with_context(|| format!("couldn't read the export at {path}"))?;
    let name = std::path::Path::new(&path).file_name().and_then(|name| name.to_str()).unwrap_or("document").to_owned();
    let key = key.map(str::trim).filter(|key| !key.is_empty()).map_or_else(|| key_for(&settings, &name), str::to_owned);
    let url = object_url(&settings, &key)?;
    let bytes = body.len();
    let signed = sign_put(&settings, &credentials, &key, &body, crate::store::now_ms())?;
    send(signed, body).await?;
    Ok(json!({"url": url.to_string(), "key": key, "format": format, "bytes": bytes}))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCESS: &str = "AKIAIOSFODNN7EXAMPLE";
    const SECRET: &str = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    /// 2013-05-24T00:00:00Z, the clock of the signatures published with the algorithm.
    const NOON: i64 = 1_369_353_600_000;

    fn sign(method: &str, path: &str, query: &str, headers: &[(&str, &str)], payload: &str) -> String {
        let body = ToSign {
            method,
            host: "examplebucket.s3.amazonaws.com",
            path,
            query,
            headers,
            payload_hash: payload,
            region: "us-east-1",
            access_key: ACCESS,
            secret_key: SECRET,
            at_ms: NOON,
        };
        body.authorization().unwrap()
    }

    #[test]
    fn the_signature_matches_the_published_get_object_example() {
        assert_eq!(
            sign("GET", "/test.txt", "", &[("range", "bytes=0-9")], EMPTY_HASH),
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,\
             SignedHeaders=host;range;x-amz-content-sha256;x-amz-date,\
             Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        );
    }

    #[test]
    fn the_signature_matches_the_published_put_object_example() {
        assert_eq!(
            sign(
                "PUT",
                "/test%24file.text",
                "",
                &[("date", "Fri, 24 May 2013 00:00:00 GMT"), ("x-amz-storage-class", "REDUCED_REDUNDANCY")],
                "44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072"
            ),
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,\
             SignedHeaders=date;host;x-amz-content-sha256;x-amz-date;x-amz-storage-class,\
             Signature=98ad721746da40c64f1a55b78f14c238d841ea1380cd77a1b5971af0ece108bd"
        );
    }

    #[test]
    fn the_signature_matches_the_published_bucket_lifecycle_example() {
        assert_eq!(
            sign("GET", "/", "lifecycle=", &[], EMPTY_HASH),
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,\
             SignedHeaders=host;x-amz-content-sha256;x-amz-date,\
             Signature=fea454ca298b7da1c68078a5d1bdbfbbe0d65c699e0f91ac7a200a0136783543"
        );
    }

    #[test]
    fn keys_escape_every_segment_and_keep_their_slashes() {
        assert_eq!(encode("test$file.text", true), "test%24file.text");
        assert_eq!(encode("carpeta/a b+c.txt", true), "carpeta/a%20b%2Bc.txt");
        assert_eq!(encode("ñ", false), "%C3%B1");
        assert_eq!(key_for(&settings(""), "Report 1.pdf"), "Report-1.pdf");
        assert_eq!(key_for(&settings("exports/2026"), "Report 1.pdf"), "exports/2026/Report-1.pdf");
    }

    fn settings(prefix: &str) -> Settings {
        Settings {
            endpoint: "https://s3.us-east-1.amazonaws.com".into(),
            region: "us-east-1".into(),
            bucket: "mybucket".into(),
            prefix: prefix.into(),
        }
    }

    fn credentials() -> Credentials {
        Credentials { access_key_id: ACCESS.into(), secret_access_key: SECRET.into() }
    }

    #[test]
    fn the_upload_url_is_path_style_and_escaped() {
        let url = object_url(&settings("exports"), "exports/carpeta/a b.txt").unwrap();
        assert_eq!(url.as_str(), "https://s3.us-east-1.amazonaws.com/mybucket/exports/carpeta/a%20b.txt");
        let signed = sign_put(&settings(""), &credentials(), "x.txt", b"hi", NOON).unwrap();
        assert_eq!(signed.url.as_str(), "https://s3.us-east-1.amazonaws.com/mybucket/x.txt");
        let header = |name: &str| signed.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str());
        assert_eq!(header(PAYLOAD_HEADER), Some(sha256_hex(b"hi").as_str()));
        assert_eq!(header("x-amz-date"), Some("20130524T000000Z"));
        assert!(header("authorization").unwrap().starts_with("AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date,Signature="));
    }

    #[test]
    fn settings_are_checked_before_they_are_kept() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert!(!status(&store).unwrap()["configured"].as_bool().unwrap());
        let mut bad = settings("");
        bad.bucket = "No Spaces".into();
        assert!(save(&store, &bad, &credentials()).is_err());
        let mut bad = settings("/x/");
        bad.prefix = "/x/".into();
        assert!(save(&store, &bad, &credentials()).is_err());
        assert!(
            save(&store, &settings("exports"), &Credentials { access_key_id: " ".into(), ..credentials() }).is_err()
        );
        save(&store, &settings("exports"), &credentials()).unwrap();
        let shown = status(&store).unwrap();
        assert_eq!(shown["configured"], true);
        assert_eq!(shown["bucket"], "mybucket");
        assert_eq!(shown["hasCredentials"], true);
        assert_eq!(shown["prefix"], "exports");
        assert!(!shown.to_string().contains(SECRET), "the secret never leaves the vault");
        disconnect(&store).unwrap();
        assert_eq!(status(&store).unwrap()["configured"], false);
    }

    #[test]
    fn a_request_without_settings_or_credentials_says_which_one_is_missing() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert!(configured(&store).unwrap_err().to_string().contains("isn't set up"));
        save(&store, &settings(""), &credentials()).unwrap();
        store.kv_delete(CREDENTIALS).unwrap();
        assert!(configured(&store).unwrap_err().to_string().contains("access key"));
    }

    /// A one-connection server: it answers with `reply` and gives back what it received.
    fn serve(reply: &'static [u8]) -> (u16, std::thread::JoinHandle<String>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut chunk = [0u8; 1024];
            while let Ok(read) = stream.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                let Some(head) = request.windows(4).position(|w| w == b"\r\n\r\n").map(|at| at + 4) else {
                    continue;
                };
                let length = String::from_utf8_lossy(&request).lines().find_map(|line| {
                    line.to_lowercase().strip_prefix("content-length: ").and_then(|n| n.trim().parse::<usize>().ok())
                });
                if length.is_none_or(|length| request.len() >= head + length) {
                    break;
                }
            }
            stream.write_all(reply).unwrap();
            String::from_utf8_lossy(&request).into_owned()
        });
        (port, handle)
    }

    #[tokio::test]
    async fn an_upload_goes_out_signed_and_carries_the_body() {
        let (port, server) = serve(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
        let settings = Settings { endpoint: format!("http://127.0.0.1:{port}"), ..settings("exports") };
        let signed = sign_put(&settings, &credentials(), "exports/hello.txt", b"hi", NOON).unwrap();
        send(signed, b"hi".to_vec()).await.unwrap();
        let request = server.join().unwrap();
        assert!(request.starts_with("PUT /mybucket/exports/hello.txt HTTP/1.1\r\n"), "{request}");
        assert!(request.to_lowercase().contains("authorization: aws4-hmac-sha256"));
        assert!(request.to_lowercase().contains("x-amz-content-sha256: "));
        assert!(request.ends_with("hi"));
    }

    #[tokio::test]
    async fn a_refused_upload_repeats_the_server_sentence() {
        let reply = b"HTTP/1.1 403 Forbidden\r\ncontent-length: 12\r\n\r\nAccessDenied";
        let (port, server) = serve(reply);
        let settings = Settings { endpoint: format!("http://127.0.0.1:{port}"), ..settings("") };
        let signed = sign_put(&settings, &credentials(), "x.txt", b"", NOON).unwrap();
        let error = send(signed, Vec::new()).await.unwrap_err().to_string();
        server.join().unwrap();
        assert!(error.contains("403") && error.contains("AccessDenied"), "{error}");
    }
}
