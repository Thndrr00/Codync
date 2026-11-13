use super::*;

#[test]
fn tokens_have_the_shape_github_uses() {
    assert!(token_ok("ghp_1234567890abcdefghijklmnopqrstuvw"));
    assert!(token_ok("github_pat_11ABCDEFG0abcdefghijklmnopqrstuvwxyz_0123456789abcdefghij"));
    for bad in ["", "short", "ghp_1234 5678", "ghp_1234.5678"] {
        assert!(!token_ok(bad), "{bad}");
    }
}

#[test]
fn the_status_never_carries_the_token() {
    let value = status(Some(&Config {
        token: "ghp_1234567890abcdefghijklmnopqrstuvw".into(),
        login: "octocat".into(),
        scopes: vec!["repo".into()],
        generation: "g".into(),
    }));
    assert_eq!(value["configured"], true);
    assert_eq!(value["login"], "octocat");
    assert!(value.get("token").is_none());
    assert_eq!(status(None)["configured"], false);
}

#[test]
fn a_repository_name_follows_githubs_rules() {
    for good in ["notes", "team-api", "site_2026", "v1.2"] {
        assert!(repo_name_ok(good), "{good}");
    }
    for bad in ["", " ", ".hidden", "with space", "slash/inside", "ümlaut", &"a".repeat(101)] {
        assert!(!repo_name_ok(bad), "{bad}");
    }
    assert!(repo_name_ok(&"a".repeat(100)));
}

#[test]
fn only_a_token_that_may_administer_offers_a_new_repository() {
    let config = |scopes: &[&str]| Config {
        token: "ghp_1234567890abcdefghijklmnopqrstuvw".into(),
        login: "octocat".into(),
        scopes: scopes.iter().map(|s| (*s).to_owned()).collect(),
        generation: "g".into(),
    };
    assert!(can_create_repos(Some(&config(&["repo"]))));
    // A fine-grained token reports no scopes, so its Administration permission is only proven by trying.
    assert!(can_create_repos(Some(&config(&[]))));
    assert!(!can_create_repos(Some(&config(&["read:user"]))));
    assert!(!can_create_repos(None));
}
