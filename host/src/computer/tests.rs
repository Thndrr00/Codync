use super::*;
use serde_json::json;

#[test]
fn omitted_capabilities_start_disabled_and_master_switch_is_authoritative() {
    let mut config: BotConfig = serde_json::from_value(json!({"id":"bot","computer":true})).unwrap();
    assert!(!Capability::Browser.enabled(&config));
    config.computer_permissions.shell = true;
    assert!(Capability::Shell.enabled(&config));
    config.computer = false;
    assert!(!Capability::Shell.enabled(&config));
}

#[tokio::test]
async fn revocation_invalidates_old_leases_without_affecting_another_capability() {
    let access = Access::default();
    let (sender, receiver) = watch::channel(0);
    access.epochs.locked().insert(("bot".into(), Capability::Shell), sender);
    let mut lease = Lease { receiver, epoch: 0 };
    access.revoke("bot", Capability::Files);
    assert!(lease.check().is_ok());
    access.revoke("bot", Capability::Shell);
    assert!(lease.check().is_err());
    tokio::time::timeout(std::time::Duration::from_millis(100), lease.cancelled()).await.unwrap();
}

#[test]
fn capability_payload_rejects_unrecognized_permission_names() {
    assert!(serde_json::from_value::<Permissions>(json!({"browser":true,"filez":true})).is_err());
}
