//! Redacted diagnostics for the peer-to-peer path a remote connection takes. Ported from OpenBot
//! 0.74.0: the app log names each STUN or TURN server a connection cannot use, with its error code,
//! and the candidate types of the path a connection uses (for example `relay/udp -> srflx/udp`).
//! It never names the address of a device: hosts of other servers and any IP address in an error
//! are replaced before the line reaches the log.

/// The scheme, host and port/transport part of a STUN or TURN URL. `None` for anything else, so an
/// unexpected value is logged as an unknown server rather than passed through.
fn server_parts(url: &str) -> Option<(&str, &str, &str)> {
    let (scheme, rest) = url.split_once(':')?;
    if !matches!(scheme.to_ascii_lowercase().as_str(), "stun" | "stuns" | "turn" | "turns") {
        return None;
    }
    let rest = rest.trim_start_matches("//");
    let (host, tail) = match rest.find([':', '?']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let host_ok = !host.is_empty()
        && host.len() <= 253
        && host.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.');
    host_ok.then_some((scheme, host, tail))
}

/// Codync's own relay hosts are safe to name; any other host is kept out of the log.
fn is_service_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "codync.run" || host.ends_with(".codync.run")
}

/// Replaces anything that looks like an IP address with `[address]`. Error text from WebRTC can
/// carry a device's local, server-reflexive or relay address; none of it belongs in the app log.
fn redact_addresses(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        if looks_like_address(word) {
            out.push_str("[address]");
        } else {
            out.push_str(word);
        }
    }
    out
}

fn looks_like_address(word: &str) -> bool {
    let core = word.trim_matches(|c: char| matches!(c, '[' | ']' | ',' | '(' | ')' | '"' | '\''));
    if core.contains(':') {
        // IPv6, possibly with a port: only hex digits, colons and dots.
        return core.bytes().all(|b| b.is_ascii_hexdigit() || matches!(b, b':' | b'.'));
    }
    // IPv4, possibly with a port: four groups of at most three digits.
    let host = core.split(':').next().unwrap_or_default();
    let groups: Vec<&str> = host.split('.').collect();
    groups.len() == 4 && groups.iter().all(|g| !g.is_empty() && g.len() <= 3 && g.bytes().all(|b| b.is_ascii_digit()))
}

/// One redacted line for a STUN or TURN server the connection could not use. The host is named only
/// when it is Codync's own; the error text keeps its words but loses every address.
pub fn server_error_detail(url: &str, error_code: i64, error_text: &str) -> String {
    let (scheme, host, tail) = match server_parts(url) {
        Some(parts) => parts,
        None => return format!("unknown server {error_code}"),
    };
    let host = if is_service_host(host) { host.to_ascii_lowercase() } else { "[host]".to_owned() };
    let text = redact_addresses(error_text);
    let text = text.trim();
    if text.is_empty() {
        format!("{scheme}:{host}{tail} {error_code}")
    } else {
        format!("{scheme}:{host}{tail} {error_code} {}", text.chars().take(120).collect::<String>())
    }
}

/// One candidate as `type/protocol`: no address, ever.
fn candidate_route(candidate: Option<(&str, &str)>) -> String {
    match candidate {
        Some((kind, protocol)) => format!("{kind}/{protocol}"),
        None => "unknown/unknown".to_owned(),
    }
}

/// The path a connection uses, such as `relay/udp -> srflx/udp`. Each side is only a candidate type
/// and a protocol, so the log says how the peers reached each other without naming either peer.
pub fn candidate_route_between(local: Option<(&str, &str)>, remote: Option<(&str, &str)>) -> String {
    format!("{} -> {}", candidate_route(local), candidate_route(remote))
}

/// The candidate type and protocol of one SDP candidate line, without its address. Used where the
/// host forwards ICE candidates: the line itself is never logged, only these two words.
pub fn sdp_candidate_kind(line: &str) -> Option<(&str, &str)> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let kind = words.iter().position(|w| w.eq_ignore_ascii_case("typ")).and_then(|i| words.get(i + 1)).copied()?;
    let protocol = words.iter().find(|w| w.eq_ignore_ascii_case("udp") || w.eq_ignore_ascii_case("tcp")).copied()?;
    Some((kind, protocol))
}

/// Logs a server a connection could not use, with its error code and no addresses.
#[allow(dead_code, reason = "the WebRTC peer that reports ICE failures lives in the clients")]
pub fn log_server_failure(url: &str, error_code: i64, error_text: &str) {
    tracing::warn!(
        detail = server_error_detail(url, error_code, error_text),
        "remote connection cannot use a STUN or TURN server"
    );
}

/// Logs the candidate types of the path a remote connection uses.
pub fn log_route(local: Option<(&str, &str)>, remote: Option<(&str, &str)>) {
    tracing::info!(route = candidate_route_between(local, remote), "remote connection path");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unusable_server_is_named_without_any_address() {
        let detail =
            server_error_detail("turn:turn.example.com:3478?transport=udp", 701, "error 192.0.2.10 from [2001:db8::1]");
        assert!(detail.starts_with("turn:[host]:3478?transport=udp 701 "), "{detail}");
        assert!(!detail.contains("192.0.2.10") && !detail.contains("2001:db8"), "{detail}");
        assert!(detail.contains("[address]"), "{detail}");

        // Codync's own relay can be named; an unrecognised URL is not passed through either.
        let own = server_error_detail("stun:stun.codync.run:3478", 400, "bad request");
        assert!(own.starts_with("stun:stun.codync.run:3478 400 bad request"), "{own}");
        assert_eq!(server_error_detail("http://example.com/path", 1, ""), "unknown server 1");
        assert_eq!(server_error_detail("turn:turn.example.com:3478", 701, " "), "turn:[host]:3478 701");
    }

    #[test]
    fn the_logged_path_has_only_candidate_types_and_protocols() {
        assert_eq!(candidate_route_between(Some(("relay", "udp")), Some(("srflx", "udp"))), "relay/udp -> srflx/udp");
        assert_eq!(candidate_route_between(None, Some(("host", "tcp"))), "unknown/unknown -> host/tcp");
        let route = candidate_route_between(Some(("relay", "udp")), Some(("srflx", "udp")));
        assert!(!route.contains('.') && !route.contains(':'), "no device address in {route}");
    }

    #[test]
    fn a_forwarded_sdp_candidate_is_logged_as_its_type_only() {
        let line = "candidate:1 1 UDP 2130706431 192.0.2.10 5000 typ srflx";
        assert_eq!(sdp_candidate_kind(line), Some(("srflx", "UDP")));
        assert!(sdp_candidate_kind("candidate:1 1 UDP 1 192.0.2.1 5000").is_none());
        assert_eq!(candidate_route_between(sdp_candidate_kind(line), Some(("relay", "udp"))), "srflx/UDP -> relay/udp");
    }
}
