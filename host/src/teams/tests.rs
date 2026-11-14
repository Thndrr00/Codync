use super::*;

#[test]
fn an_app_id_is_a_guid() {
    assert_eq!(app_id(" 00000000-0000-0000-0000-000000000001 ").unwrap(), "00000000-0000-0000-0000-000000000001");
    assert!(app_id("not-a-guid").is_err());
    assert!(app_id("00000000-0000-0000-0000-00000000000").is_err());
}

#[test]
fn a_tenant_is_optional_and_has_no_credentials() {
    assert_eq!(tenant("").unwrap(), "");
    assert_eq!(tenant("contoso.onmicrosoft.com").unwrap(), "contoso.onmicrosoft.com");
    assert!(tenant("a tenant").is_err());
}

#[test]
fn allowed_lists_are_trimmed_and_bounded() {
    let list = people(&[" ana@example.com ".into(), "".into(), "*".into()], "people").unwrap();
    assert_eq!(list, ["ana@example.com", "*"]);
    let many: Vec<String> = (0..MAX_ALLOWED + 1).map(|i| format!("u{i}")).collect();
    assert!(people(&many, "people").is_err());
}

#[test]
fn the_registration_names_the_public_endpoint_and_the_azure_bot() {
    let text = registration("/hooks/teams/abc");
    assert!(text.contains("https://<this computer's public host>/hooks/teams/abc"));
    assert!(text.contains("Azure Bot resource"));
    assert!(text.contains("Microsoft Teams channel"));
}
