use super::*;

#[test]
fn a_number_is_e164_and_a_chat_is_a_number_or_a_group() {
    assert_eq!(number(" +15551234567 ").unwrap(), "+15551234567");
    assert!(number("5551234567").is_err());
    assert!(number("+05551234567").is_err());
    assert!(number("+1555 123 4567").is_err());
    assert_eq!(chat("+15551234567").unwrap(), "+15551234567");
    assert_eq!(chat("group:abc+/=").unwrap(), "group:abc+/=");
    assert!(chat("group:").is_err());
    assert!(chat("team").is_err());
}

#[test]
fn a_daemon_address_is_a_url_without_a_path() {
    assert_eq!(daemon("http://127.0.0.1:7583/").unwrap(), "http://127.0.0.1:7583");
    assert!(daemon("ftp://127.0.0.1").is_err());
    assert!(daemon("http://user:pass@127.0.0.1:7583").is_err());
    assert!(daemon("http://127.0.0.1:7583/api").is_err());
}

#[test]
fn the_setup_line_names_the_real_commands() {
    let text = setup_text("+15551234567");
    assert!(text.contains("signal-cli -a +15551234567 link"));
    assert!(text.contains("daemon --http 127.0.0.1:7583 --receive-mode=on-start"));
    assert!(!text.contains("connected"));
}
