#![cfg(feature = "nostr-client")]

use radroots_field_core::RadrootsRuntime;

#[test]
fn identity_reset_all_removes_selected_and_unselected_identities() {
    let runtime = RadrootsRuntime::new().expect("runtime");

    let selected_id = runtime
        .nostr_identity_generate(Some("selected".to_string()), true)
        .expect("selected identity");
    let other_id = runtime
        .nostr_identity_generate(Some("other".to_string()), false)
        .expect("other identity");

    let snapshot = runtime.nostr_identity_snapshot().expect("snapshot");
    assert!(snapshot.has_selected_signing_identity);
    assert_eq!(
        snapshot.selected_identity_id.as_deref(),
        Some(selected_id.as_str())
    );
    assert_eq!(snapshot.identities.len(), 2);
    assert!(
        runtime
            .nostr_identity_export_selected_secret_hex()
            .expect("export")
            .is_some()
    );

    runtime.nostr_identity_reset_all().expect("reset all");

    let snapshot = runtime.nostr_identity_snapshot().expect("reset snapshot");
    assert!(!snapshot.has_selected_signing_identity);
    assert_eq!(snapshot.selected_identity_id, None);
    assert_eq!(snapshot.selected_npub, None);
    assert!(snapshot.identities.is_empty());
    assert!(runtime.nostr_identity_list().expect("list").is_empty());
    assert_eq!(
        runtime
            .nostr_identity_export_selected_secret_hex()
            .expect("export after reset"),
        None
    );

    assert!(runtime.nostr_identity_remove(other_id).is_err());
}
