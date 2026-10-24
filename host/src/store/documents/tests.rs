use super::*;
use crate::store::tests::temp_store;

fn bot(store: &Store, id: &str) {
    let config = serde_json::from_value(serde_json::json!({"id": id, "name": id})).unwrap();
    store.save_bot(&config).unwrap();
}

fn draft(title: &str, body: &str) -> DocumentDraft {
    DocumentDraft { id: None, parent_id: None, title: title.into(), body: body.into(), if_revision: 0 }
}

fn saved(result: DocumentWrite) -> Document {
    match result {
        DocumentWrite::Saved(doc) => doc,
        DocumentWrite::Conflict(_) => panic!("unexpected conflict"),
    }
}

#[test]
fn pending_reviews_remain_visible_after_more_than_one_hundred_newer_proposals() {
    let store = temp_store();
    bot(&store, "a");
    let oldest = store.review_document("a", draft("Old proposal", "body"), Some("oldest")).unwrap();
    for index in 0..105 {
        store.review_document("a", draft(&format!("Proposal {index}"), "body"), Some(&format!("new-{index}"))).unwrap();
    }
    let reviews = store.document_reviews("a").unwrap();
    assert_eq!(reviews.len(), 106);
    assert!(reviews.iter().any(|review| review.id == oldest.id));
}

#[test]
fn completed_history_does_not_exhaust_future_creation_or_review() {
    let store = temp_store();
    bot(&store, "a");
    let original = saved(store.save_document("a", draft("Existing", "body")).unwrap());
    {
        let db = store.db.locked();
        let payload = serde_json::to_string(&draft("Old proposal", "body")).unwrap();
        let document = serde_json::to_string(&original).unwrap();
        for index in 0..1000 {
            db.execute(
                "INSERT INTO document_creations VALUES('a',?1,?2)",
                rusqlite::params![format!("old-{index}"), document],
            )
            .unwrap();
            db.execute(
                "INSERT INTO document_reviews(id,bot_id,effect_key,payload_hash,payload,status,created_at,expires_at)
                VALUES(?1,'a',?1,'hash',?2,'declined',1,2)",
                rusqlite::params![format!("old-{index}"), payload],
            )
            .unwrap();
        }
    }
    assert!(matches!(
        store.save_document_once("a", draft("New page", "new"), Some("fresh")).unwrap(),
        DocumentWrite::Saved(_)
    ));
    assert_eq!(
        store.review_document("a", draft("New review", "new"), Some("fresh-review")).unwrap().status,
        ReviewStatus::Pending
    );
}

#[test]
fn documents_preserve_markdown_and_reject_stale_writes_and_hierarchy_cycles() {
    let store = temp_store();
    bot(&store, "a");
    bot(&store, "b");
    let parent = saved(store.save_document("a", draft("ÁRBOL", "# Heading\r\n\n- 日本語 🐱\n")).unwrap());
    let mut child = draft("Child", "body");
    child.parent_id = Some(parent.id.clone());
    let child = saved(store.save_document("a", child).unwrap());
    assert!(store.move_document(&parent.id, Some(&child.id), 1).is_err());
    assert!(store.move_document(&parent.id, Some(&parent.id), 1).is_err());
    let other = saved(store.save_document("b", draft("Other", "private")).unwrap());
    assert!(store.move_document(&child.id, Some(&other.id), 1).is_err());
    assert_eq!(store.documents("a", "árbol", None, 100).unwrap().items.len(), 1);
    assert_eq!(store.documents("b", "árbol", None, 100).unwrap().items.len(), 0);
    let update = DocumentDraft {
        id: Some(parent.id.clone()),
        parent_id: None,
        if_revision: 1,
        title: parent.title.clone(),
        body: "new content".into(),
    };
    let updated = saved(store.save_document("a", update.clone()).unwrap());
    assert_eq!(updated.revision, 2);
    match store.save_document("a", update).unwrap() {
        DocumentWrite::Conflict(current) => assert_eq!(current.body, "new content"),
        DocumentWrite::Saved(_) => panic!("stale write must conflict"),
    }
    assert!(
        store
            .save_document("b", DocumentDraft { id: Some(parent.id.clone()), if_revision: 2, ..draft("Stolen", "no") })
            .is_err()
    );
    assert_eq!(store.document(&parent.id).unwrap().unwrap().body, "new content");
    assert!(matches!(store.delete_document(&parent.id, 1).unwrap(), DocumentDelete::Conflict { .. }));
    let deleted = store.delete_document(&parent.id, 2).unwrap();
    assert!(matches!(deleted, DocumentDelete::Deleted { ids, .. } if ids.len() == 2));
    assert!(store.document(&child.id).unwrap().is_none());
    assert!(store.document(&other.id).unwrap().is_some());
}

#[test]
fn documents_and_immutable_approval_results_survive_restart() {
    let dir = std::env::temp_dir().join(format!("codync-docs-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("host.db");
    let (review_id, original) = {
        let store = Store::open(&path).unwrap();
        bot(&store, "a");
        let review = store.review_document("a", draft("Report", "# UTF-8\r\n🦉\n"), Some("request-1")).unwrap();
        assert!(store.documents("a", "", None, 100).unwrap().items.is_empty());
        let repeated = store.review_document("a", draft("Report", "# UTF-8\r\n🦉\n"), Some("request-1")).unwrap();
        assert_eq!(review.id, repeated.id);
        assert!(store.review_document("a", draft("Report", "different"), Some("request-1")).is_err());
        let approved = store.resolve_document_review(&review.id, ReviewDecision::Approve).unwrap();
        let original = approved.document.unwrap();
        assert_eq!(original.body, "# UTF-8\r\n🦉\n");
        assert_eq!(store.documents("a", "", None, 100).unwrap().items.len(), 1);
        (review.id, original)
    };
    {
        let store = Store::open(&path).unwrap();
        let repeat = store.resolve_document_review(&review_id, ReviewDecision::Approve).unwrap();
        assert_eq!(repeat.document.as_ref(), Some(&original));
        saved(
            store
                .save_document(
                    "a",
                    DocumentDraft {
                        id: Some(original.id.clone()),
                        if_revision: 1,
                        ..draft("Changed", "newer version")
                    },
                )
                .unwrap(),
        );
        let repeat = store.resolve_document_review(&review_id, ReviewDecision::Decline).unwrap();
        assert_eq!(repeat.status, ReviewStatus::Approved);
        assert_eq!(repeat.document.as_ref(), Some(&original), "retry returns original snapshot, not latest edit");
        assert_eq!(store.documents("a", "", None, 100).unwrap().items.len(), 1);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn review_decline_expiration_and_concurrent_edits_never_save_the_draft() {
    let store = temp_store();
    bot(&store, "a");
    let declined = store.review_document("a", draft("Decline", "not saved"), None).unwrap();
    assert_eq!(
        store.resolve_document_review(&declined.id, ReviewDecision::Decline).unwrap().status,
        ReviewStatus::Declined
    );
    assert_eq!(
        store.resolve_document_review(&declined.id, ReviewDecision::Approve).unwrap().status,
        ReviewStatus::Declined
    );
    let expired = store.review_document("a", draft("Expires", "not saved"), None).unwrap();
    store.db.locked().execute("UPDATE document_reviews SET expires_at = 0 WHERE id = ?1", [&expired.id]).unwrap();
    assert_eq!(
        store.resolve_document_review(&expired.id, ReviewDecision::Approve).unwrap().status,
        ReviewStatus::Expired
    );
    assert!(store.documents("a", "", None, 100).unwrap().items.is_empty());
    let doc = saved(store.save_document("a", draft("Existing", "before")).unwrap());
    let proposal = DocumentDraft { id: Some(doc.id.clone()), if_revision: 1, ..draft("Reviewed", "agent draft") };
    let review = store.review_document("a", proposal, None).unwrap();
    saved(
        store
            .save_document(
                "a",
                DocumentDraft { id: Some(doc.id.clone()), if_revision: 1, ..draft("User edit", "user's new text") },
            )
            .unwrap(),
    );
    let resolved = store.resolve_document_review(&review.id, ReviewDecision::Approve).unwrap();
    assert_eq!(resolved.status, ReviewStatus::Conflict);
    assert_eq!(store.document(&doc.id).unwrap().unwrap().body, "user's new text");
    store.delete_bot("a").unwrap();
    assert!(store.document_review(&review.id).unwrap().is_none());
    assert!(store.document(&doc.id).unwrap().is_none());
}

#[test]
fn document_pages_are_bounded_and_body_size_is_validated() {
    let store = temp_store();
    bot(&store, "a");
    for title in ["one", "two", "three"] {
        store.save_document("a", draft(title, "body")).unwrap();
    }
    let page = store.documents("a", "", None, 2).unwrap();
    assert_eq!(page.items.len(), 2);
    let second = store.documents("a", "", page.next.as_deref(), 2).unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(second.next.is_none());
    assert!(!page.items.iter().any(|item| item.id == second.items[0].id));
    assert!(store.save_document("a", draft("", "body")).is_err());
    assert!(store.save_document("a", draft("Too large", &"x".repeat(types::MAX_BODY_BYTES + 1))).is_err());
    assert!(store.save_document("missing", draft("Unknown", "body")).is_err());
}

#[test]
fn creation_nonce_returns_original_document_without_overwriting_newer_edits() {
    let store = temp_store();
    bot(&store, "a");
    let original = saved(store.save_document_once("a", draft("Original", "first"), Some("create-request")).unwrap());
    let newer = saved(
        store
            .save_document(
                "a",
                DocumentDraft { id: Some(original.id.clone()), if_revision: 1, ..draft("Newer", "user edited") },
            )
            .unwrap(),
    );
    let retried = saved(
        store.save_document_once("a", draft("Uncertain retry", "do not write twice"), Some("create-request")).unwrap(),
    );
    assert_eq!(retried, original);
    assert_eq!(store.documents("a", "", None, 100).unwrap().items.len(), 1);
    assert_eq!(store.document(&newer.id).unwrap().unwrap(), newer);
}
