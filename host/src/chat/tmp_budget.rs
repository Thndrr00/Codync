//! One memory budget for all agents' /tmp; idle leftovers go when memory is short (ADR 0243).
//!
//! A tmpfs capped per agent has no total cap: several agents working for days hold gigabytes the
//! kernel cannot drop. This is the pool rule, sized as a share of RAM, with the action to take when
//! the machine is short. Codync already reads RAM in `health`; no new daemon needed here.

use serde_json::{Value, json};

/// The pool is at most this share of total RAM (ADR 0243: 20%, i.e. 1/5).
pub const POOL_SHARE_DIVISOR: u64 = 5;

/// Pool bytes, as a share of total RAM, rounded down to whole MiB. The share is 20% (one fifth), so
/// integer division keeps the pool bounded without a lossy float cast.
pub fn pool_bytes(total_ram_bytes: u64) -> u64 {
    let bytes = total_ram_bytes / POOL_SHARE_DIVISOR;
    bytes / (1024 * 1024) * (1024 * 1024)
}

/// What to do at a given free-RAM ratio: clear idle tmp, then park idle desktops, then nothing more
/// we can do automatically (the person is told). Ordered so the least disruptive action comes first.
pub fn pressure(free_ratio: f64, idle_tmp_bytes: u64, idle_desktops: usize) -> Value {
    let action = if free_ratio < 0.05 {
        if idle_desktops > 0 { "park_idle_desktops" } else { "ask_person" }
    } else if free_ratio < 0.15 && idle_tmp_bytes > 0 {
        "clear_idle_tmp"
    } else {
        "none"
    };
    json!({"action": action, "idleTmpBytes": idle_tmp_bytes, "idleDesktops": idle_desktops})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pool_is_bounded_and_pressure_escalates() {
        let total = 8 * 1024 * 1024 * 1024;
        let expected = (total / POOL_SHARE_DIVISOR) / (1024 * 1024) * (1024 * 1024);
        assert_eq!(pool_bytes(total), expected);
        assert_eq!(pressure(0.5, 0, 0)["action"], "none");
        assert_eq!(pressure(0.1, 1000, 0)["action"], "clear_idle_tmp");
        assert_eq!(pressure(0.02, 0, 2)["action"], "park_idle_desktops");
        assert_eq!(pressure(0.02, 0, 0)["action"], "ask_person");
    }
}
