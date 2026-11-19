//! Time-based one-time passwords for logins in the vault (RFC 6238, SHA-256, six digits).
//! The base32 seed stays in the encrypted record and on this host; a bot only ever receives
//! the current code and the seconds until it changes.

use crate::LockExt;
use anyhow::{Result, anyhow, bail};
use hmac::{KeyInit as _, Mac as _};
use sha2::{Digest as _, Sha256};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Seconds a code stays current.
pub const STEP: u64 = 30;
/// Six digits, as every authenticator app shows them.
const DIGITS: u32 = 6;
/// One entry per seed per step; older steps are dropped together when this fills up.
const MAX_CACHE: usize = 256;

static CACHE: LazyLock<std::sync::Mutex<HashMap<(u64, [u8; 8]), String>>> = LazyLock::new(Mutex::default);

/// RFC 4648's alphabet, without the padding character.
fn symbol(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a'),
        b'2'..=b'7' => Some(byte - b'2' + 26),
        _ => None,
    }
}

/// Decodes a base32 secret, accepting lower case, spaces, hyphens and trailing `=`.
pub fn base32(input: &str) -> Result<Vec<u8>> {
    if input.len() > 256 {
        bail!("The 2FA secret is too long");
    }
    let mut out = Vec::with_capacity(input.len() * 5 / 8);
    let mut bits = 0u32;
    let mut left = 0u32;
    let mut padding = false;
    for byte in input.bytes() {
        if byte == b'=' {
            padding = true;
            continue;
        }
        if byte.is_ascii_whitespace() || byte == b'-' {
            continue;
        }
        if padding {
            bail!("The 2FA secret has characters after its padding");
        }
        let value = symbol(byte).ok_or_else(|| anyhow!("The 2FA secret must be base32: A-Z and 2-7"))?;
        bits = (bits << 5) | u32::from(value);
        left += 5;
        if left >= 8 {
            left -= 8;
            out.push((bits >> left) as u8);
            bits &= (1 << left) - 1;
        }
    }
    // 5, 6 and 7 trailing bits are not a whole base32 value, nor are non-zero leftover bits.
    if matches!(left, 5 | 6 | 7) || bits != 0 {
        bail!("The 2FA secret isn't valid base32");
    }
    Ok(out)
}

/// Refuses a seed the host couldn't turn into a code, before it reaches the vault.
pub fn check_secret(secret: &str) -> Result<()> {
    if base32(secret)?.len() < 10 {
        bail!("The 2FA secret is too short: use at least 16 base32 characters");
    }
    Ok(())
}

pub fn unix_seconds() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn step(now: u64) -> u64 {
    now / STEP
}

/// The dynamic truncation of RFC 4226, parameterised by digit count so the RFC's eight-digit
/// vectors can be checked while the tool returns the six digits people recognise.
fn generate(secret: &str, counter: u64, digits: u32) -> Result<String> {
    let key = base32(secret)?;
    let mut mac = hmac::Hmac::<Sha256>::new_from_slice(&key).map_err(|_| anyhow!("The 2FA secret can't be used"))?;
    mac.update(&counter.to_be_bytes());
    let tag = mac.finalize().into_bytes();
    let offset = usize::from(tag[31] & 0x0f);
    let binary = u32::from_be_bytes([tag[offset] & 0x7f, tag[offset + 1], tag[offset + 2], tag[offset + 3]]);
    Ok(format!("{:0width$}", binary % 10u32.pow(digits), width = digits as usize))
}

/// The current code and how long it stays current. Cached by seed fingerprint and step, so
/// repeated calls in one window don't recompute and the cache holds no secret in the clear.
pub fn code_for(secret: &str, now: u64) -> Result<(String, u64)> {
    let counter = step(now);
    let digest = Sha256::digest(secret.as_bytes());
    let mut fingerprint = [0u8; 8];
    fingerprint.copy_from_slice(&digest[..8]);
    let key = (counter, fingerprint);
    let mut cache = CACHE.locked();
    if let Some(code) = cache.get(&key) {
        return Ok((code.clone(), STEP - now % STEP));
    }
    let code = generate(secret, counter, DIGITS)?;
    if cache.len() >= MAX_CACHE {
        cache.clear();
    }
    cache.insert(key, code.clone());
    Ok((code, STEP - now % STEP))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_decodes_spaces_case_and_padding() {
        assert_eq!(base32("GEZDGNBVGY3TQOJQ").unwrap(), b"1234567890");
        assert_eq!(base32("gezd gnbv gy3t qojq").unwrap(), b"1234567890");
        assert_eq!(base32("MZXW6===").unwrap(), b"foo");
    }

    #[test]
    fn bad_base32_is_rejected_clearly() {
        assert!(base32("GEZDGNBVGY3TQOJ1").unwrap_err().to_string().contains("base32"));
        assert!(base32("GEZDGNBVGY3TQOJQ=A").is_err());
        assert!(base32("A").is_err());
        assert!(check_secret("GEZDGNBV").is_err());
        assert!(check_secret("GEZDGNBVGY3TQOJQ").is_ok());
    }

    #[test]
    fn codes_change_on_the_thirty_second_step() {
        assert_eq!(step(29), 0);
        assert_eq!(step(30), 1);
        assert_eq!(base32("GEZDGNBVGY3TQOJQ").unwrap().len(), 10);
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        assert_eq!(code_for(secret, 59).unwrap().1, 1);
        assert_eq!(code_for(secret, 60).unwrap().1, STEP);
        assert_ne!(code_for(secret, 59).unwrap().0, code_for(secret, 60).unwrap().0);
        assert_eq!(code_for(secret, 61).unwrap(), code_for(secret, 61).unwrap());
    }

    #[test]
    fn rfc6238_sha256_vector() {
        // ASCII "12345678901234567890123456789012", the RFC 6238 SHA-256 test key.
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQGEZA";
        assert_eq!(generate(secret, step(59), 8).unwrap(), "46119246");
        assert_eq!(generate(secret, step(1_111_111_109), 8).unwrap(), "68084774");
        assert_eq!(generate(secret, step(1_111_111_111), 8).unwrap(), "67062674");
        // Bots get six digits: the same vector truncated to six.
        assert_eq!(code_for(secret, 59).unwrap().0, "119246");
    }
}
