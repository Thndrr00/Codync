//! Domain-allowlist egress gate (`egress.rs`) for sandboxed bots.
//!
//! Validates outbound `host:port` targets against the plan's `allowed_egress_domains`
//! (supporting exact domain matches and `*.example.com` subdomain wildcards) while
//! always refusing loopback or RFC1918 private network addresses so a boxed bot cannot
//! probe the host's local services.

use std::net::IpAddr;

/// Checks whether a destination `host` is permitted by `allowed_domains`.
/// Loopback (`localhost`, `127.0.0.1`, `::1`) and private LAN literals are always denied.
pub fn is_egress_allowed(host: &str, allowed_domains: &[String]) -> bool {
    let clean = host
        .strip_prefix('[')
        .and_then(|s| s.split_once(']').map(|(h, _)| h))
        .unwrap_or_else(|| host.rsplit_once(':').map_or(host, |(h, _)| h))
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if clean.is_empty() || clean == "localhost" {
        return false;
    }
    if let Ok(ip) = clean.parse::<IpAddr>() {
        let canonical = ip.to_canonical();
        if canonical.is_loopback() || canonical.is_unspecified() {
            return false;
        }
        if let IpAddr::V4(v4) = canonical
            && (v4.is_private() || v4.is_link_local())
        {
            return false;
        }
        return allowed_domains.iter().any(|d| d.trim().eq_ignore_ascii_case(&clean));
    }
    if allowed_domains.is_empty() {
        return false;
    }
    allowed_domains.iter().any(|pattern| {
        let pat = pattern.trim().trim_end_matches('.').to_ascii_lowercase();
        if pat == "*" {
            return true;
        }
        if let Some(suffix) = pat.strip_prefix("*.") {
            clean == suffix || clean.ends_with(&format!(".{suffix}"))
        } else {
            clean == pat
        }
    })
}
