//! Personal mode: the same workspace, shaped for one person (agentes.chat ADR 0159).
//!
//! A single-person workspace hides team surfaces (roles, invitations, audiences) and shows a
//! simpler shape. Codync is already single-user; this is the flag the clients can read to pick the
//! simple layout without the host growing a team model it does not need.

use serde_json::{Value, json};

/// The mode for a workspace: personal by default, team only when it really has several people.
pub fn mode(members: usize) -> Value {
    let personal = members <= 1;
    json!({
        "mode": if personal { "personal" } else { "team" },
        "showTeamSurfaces": !personal,
        "members": members,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_member_is_personal_and_hides_team_ui() {
        assert_eq!(mode(1)["mode"], "personal");
        assert_eq!(mode(1)["showTeamSurfaces"], false);
        assert_eq!(mode(4)["mode"], "team");
        assert_eq!(mode(4)["showTeamSurfaces"], true);
    }
}
