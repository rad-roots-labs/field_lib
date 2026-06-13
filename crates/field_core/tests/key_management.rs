#![cfg(feature = "nostr-client")]

use radroots_field_core::RadrootsRuntime;

#[test]
fn identity_reset_all_removes_selected_and_unselected_identities() {
    let runtime = RadrootsRuntime::new().expect("runtime");

    let selected = runtime
        .nostr_identity_generate(Some("selected".to_string()), true)
        .expect("selected identity");
    let other = runtime
        .nostr_identity_generate(Some("other".to_string()), false)
        .expect("other identity");

    let snapshot = runtime.nostr_identity_snapshot().expect("snapshot");
    assert!(snapshot.has_selected_signing_identity);
    assert_eq!(
        snapshot.selected_identity_id.as_deref(),
        Some(selected.id.as_str())
    );
    assert!(selected.is_selected);
    assert!(!other.is_selected);
    assert_eq!(snapshot.identities.len(), 2);

    runtime.nostr_identity_reset_all().expect("reset all");

    let snapshot = runtime.nostr_identity_snapshot().expect("reset snapshot");
    assert!(!snapshot.has_selected_signing_identity);
    assert_eq!(snapshot.selected_identity_id, None);
    assert_eq!(snapshot.selected_npub, None);
    assert!(snapshot.identities.is_empty());
    assert!(runtime.nostr_identity_list().expect("list").is_empty());

    assert!(runtime.nostr_identity_remove(other.id).is_err());
}

#[test]
fn host_secret_restore_recreates_runtime_signing_identity_after_lock() {
    let runtime = RadrootsRuntime::new().expect("runtime");
    let host_identity = radroots_identity::RadrootsIdentity::generate();
    let secret_key = host_identity.secret_key_hex();

    let restored = runtime
        .nostr_identity_restore_host_secret(
            secret_key.clone(),
            Some("local custody".to_string()),
            true,
        )
        .expect("restore host secret");
    assert_eq!(restored.public_key_hex, host_identity.public_key_hex());
    assert!(restored.is_selected);
    assert!(runtime.nostr_identity_has_selected_signing_identity());

    runtime
        .nostr_identity_clear_runtime_state()
        .expect("clear runtime state");
    let locked = runtime.nostr_identity_snapshot().expect("locked snapshot");
    assert!(!locked.has_selected_signing_identity);
    assert!(locked.identities.is_empty());
    assert!(!runtime.nostr_identity_has_selected_signing_identity());

    let restored_again = runtime
        .nostr_identity_restore_host_secret(secret_key, Some("local custody".to_string()), true)
        .expect("restore host secret again");
    assert_eq!(
        restored_again.public_key_hex,
        host_identity.public_key_hex()
    );
    assert!(runtime.nostr_identity_has_selected_signing_identity());
}
