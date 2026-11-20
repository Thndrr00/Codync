//! Share ids and link refs: strict, origin-bound and safe as a path segment.

use super::{ID_LEN, SHARE_ORIGIN, SHARE_PATH};
use anyhow::{Result, bail};
use base64::Engine as _;

/// Only ASCII letters, digits, `_` and `-`, exactly 22 characters long: safe as
/// a link segment, a store id and a file name, with no path or encoding tricks.
pub fn is_template_id(id: &str) -> bool {
    id.len() == ID_LEN && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// A fresh, unguessable share id: 16 random bytes in base64 without padding.
pub fn new_template_id() -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(uuid::Uuid::new_v4().as_bytes())
}

pub(crate) fn checked_id(id: &str) -> Result<String> {
    if is_template_id(id) { Ok(id.to_owned()) } else { bail!("invalid agent template id") }
}

/// The public link of a published template.
pub fn share_url(id: &str) -> String {
    format!("{SHARE_ORIGIN}{SHARE_PATH}{id}")
}

fn is_known_web_origin(authority: &str, scheme: &str) -> bool {
    if authority == "codync.dev" {
        return scheme == "https";
    }
    let host = authority.split_once(':').map_or(authority, |(host, _)| host);
    (host == "localhost" || host == "127.0.0.1")
        && scheme == "http"
        && authority.split_once(':').is_none_or(|(_, port)| port.parse::<u16>().is_ok())
}

/// Accepts a bare id, a `https://codync.dev/agents/<id>` link (or a local dev
/// origin) and a `codync://agents/<id>` link. Every other origin is refused.
pub fn parse_template_ref(value: &str) -> Result<String> {
    let value = value.trim();
    if is_template_id(value) {
        return Ok(value.to_owned());
    }
    let Some((scheme, rest)) = value.split_once("://") else {
        bail!("unknown agent template link origin");
    };
    let id = match scheme {
        "https" | "http" => {
            let Some((authority, path)) = rest.split_once('/') else {
                bail!("unknown agent template link origin");
            };
            if !is_known_web_origin(authority, scheme) {
                bail!("unknown agent template link origin");
            }
            let Some(id) = path.strip_prefix(SHARE_PATH.trim_start_matches('/')) else {
                bail!("that isn't an agent template link");
            };
            id
        }
        "codync" => {
            let Some((host, id)) = rest.split_once('/') else {
                bail!("that isn't an agent template link");
            };
            if host != "agents" {
                bail!("unknown agent template link origin");
            }
            id
        }
        _ => bail!("unknown agent template link origin"),
    };
    if !is_template_id(id) {
        bail!("that isn't an agent template link");
    }
    Ok(id.to_owned())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const ID: &str = "AbCdEf0123456789_xYz-0"; // 22 safe characters

    #[test]
    fn ids_are_strict_and_generated_ids_are_valid() {
        assert!(is_template_id(ID));
        assert_eq!(ID.len(), ID_LEN);
        assert!(is_template_id(&new_template_id()));
        for bad in [
            "",
            "short",
            "AbCdEf0123456789_xYz",    // 21
            "AbCdEf0123456789_xYz-00", // 23
            "AbCdEf0123456789 xYz-0",  // space
            "AbCdEf0123456789/xYz-0",  // path separator
            "AbCdEf0123456789.xYz-0",  // dot
            "AbCdEf0123456789_xYé-0",  // non-ASCII
        ] {
            assert!(!is_template_id(bad), "{bad}");
        }
    }

    #[test]
    fn share_refs_refuse_unknown_origins() {
        let url = share_url(ID);
        let custom = format!("codync://agents/{ID}");
        let local = format!("http://localhost/agents/{ID}");
        assert_eq!(url, format!("https://codync.dev/agents/{ID}"));
        for good in [ID, url.as_str(), custom.as_str(), local.as_str()] {
            assert_eq!(parse_template_ref(good).unwrap(), ID, "{good}");
        }
        for bad in [
            "https://evil.example/agents/AbCdEf0123456789_xYz-0",
            "http://codync.dev/agents/AbCdEf0123456789_xYz-0",
            "openbot://agents/AbCdEf0123456789_xYz-0",
            "codync://bots/AbCdEf0123456789_xYz-0",
            "https://user@codync.dev/agents/AbCdEf0123456789_xYz-0",
            "https://codync.dev:8443/agents/AbCdEf0123456789_xYz-0",
            "https://codync.dev/agents/AbCdEf0123456789_xYz-0/extra",
            "https://codync.dev/agents/AbCdEf0123456789_xYz-0?x=1",
            "https://codync.dev/agents/AbCdEf0123456789_xYz-0#f",
            "not a ref",
        ] {
            assert!(parse_template_ref(bad).is_err(), "{bad}");
        }
    }
}
