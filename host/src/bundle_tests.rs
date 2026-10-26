//! Bundle import behavior, including failures after persistence has begun.

use super::*;
use crate::LockExt;
use crate::remote::identity::Identity;
use crate::store::Store;
use std::path::PathBuf;

struct Fixture {
    hub: Arc<Hub>,
    dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("codync-bundle-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        // Exercise default connector setup without accessing the person's credential manager.
        *store.secret_key.locked() = Some(zeroize::Zeroizing::new(vec![0; 32]));
        let hub = Hub::new(store, "bundle-test".into(), Identity::load_or_create(&dir).unwrap(), "test".into(), 0);
        Self { hub, dir }
    }

    fn bundle(second_backend: &str) -> Value {
        json!({
            "format": FORMAT, "version": VERSION,
            "bots": [
                {"name": "First", "backend": "claude"},
                {"name": "Second", "backend": second_backend},
            ],
        })
    }

    fn fail_second_insert(&self) {
        self.hub
            .store
            .with_connection(|db| {
                db.execute_batch(
                    "CREATE TRIGGER reject_second BEFORE INSERT ON bots
                 WHEN json_extract(NEW.config, '$.name') = 'Second'
                 BEGIN SELECT RAISE(FAIL, 'injected import failure'); END;",
                )?;
                Ok(())
            })
            .unwrap();
    }

    async fn import(&self, bundle: &Value) -> Result<Value> {
        import(&self.hub, bundle, self.dir.to_str().unwrap()).await
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[tokio::test]
async fn invalid_later_bot_leaves_no_imported_bots() {
    let fixture = Fixture::new();
    let error = fixture.import(&Fixture::bundle("unknown-bundle-agent")).await.unwrap_err();
    assert!(format!("{error:#}").contains("unknown backend"));
    assert!(fixture.hub.store.bots().unwrap().is_empty());
}

#[tokio::test]
async fn failed_later_insert_removes_the_bots_already_created() {
    let fixture = Fixture::new();
    fixture.fail_second_insert();
    let error = fixture.import(&Fixture::bundle("claude")).await.unwrap_err();
    assert!(format!("{error:#}").contains("injected import failure"));
    let bots = fixture.hub.store.bots().unwrap();
    assert_eq!(bots.len(), 1, "the first bot reached persistence before the failure");
    assert!(bots[0].deleted, "the failed import must not leave a live bot");
}

#[tokio::test]
async fn incomplete_rollback_is_reported_to_the_importer() {
    let fixture = Fixture::new();
    fixture.fail_second_insert();
    fixture
        .hub
        .store
        .with_connection(|db| {
            db.execute_batch(
                "CREATE TRIGGER reject_cleanup BEFORE UPDATE OF deleted ON bots
             WHEN NEW.deleted = 1
             BEGIN SELECT RAISE(FAIL, 'injected cleanup failure'); END;",
            )?;
            Ok(())
        })
        .unwrap();
    let error = fixture.import(&Fixture::bundle("claude")).await.unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("injected import failure"));
    assert!(message.contains("Could not remove imported bots"));
    assert!(message.contains("injected cleanup failure"));
    let bots = fixture.hub.store.bots().unwrap();
    assert_eq!(bots.len(), 1);
    assert!(!bots[0].deleted);
    assert!(message.contains(&bots[0].config.id), "the user can identify the retained bot");
}
