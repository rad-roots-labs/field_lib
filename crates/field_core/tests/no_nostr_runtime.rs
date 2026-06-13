#![cfg(not(feature = "nostr-client"))]

use radroots_field_core::logging;
use radroots_field_core::runtime::builder::RuntimeBuilder;
use radroots_field_core::runtime::nostr::{
    NostrConnectionStatus, NostrEvent, NostrLight, NostrPost, NostrPostEventMetadata, NostrProfile,
    NostrProfileEventMetadata,
};
use radroots_field_core::{RadrootsAppError, RadrootsRuntime};
use radroots_net_core::config::NetConfig;

fn expect_disabled<T>(result: Result<T, RadrootsAppError>) {
    match result {
        Err(RadrootsAppError::Msg(message)) => assert_eq!(message, "nostr disabled"),
        _ => panic!("expected nostr disabled error"),
    }
}

#[test]
fn runtime_info_and_platform_paths_are_exercised() {
    let runtime = RadrootsRuntime::new().expect("runtime");
    assert!(runtime.uptime_millis() >= 0);

    runtime.stop();
    runtime.stop();
}

#[test]
fn key_management_disabled_paths_are_exercised() {
    let runtime = RadrootsRuntime::new().expect("runtime");

    assert!(!runtime.nostr_identity_has_selected_signing_identity());
    assert_eq!(runtime.nostr_identity_selected_npub(), None);
    expect_disabled(runtime.nostr_identity_list());
    expect_disabled(runtime.nostr_identity_list_ids());
    expect_disabled(runtime.nostr_identity_snapshot());
    expect_disabled(runtime.nostr_identity_validate_host_custody_secret("deadbeef".to_string()));
    expect_disabled(runtime.nostr_identity_restore_host_custody_secret(
        "deadbeef".to_string(),
        Some("alpha".to_string()),
        true,
    ));
    expect_disabled(runtime.nostr_identity_select("account-1".to_string()));
    expect_disabled(runtime.nostr_identity_remove("account-1".to_string()));
    expect_disabled(runtime.nostr_identity_lock_host_custody_runtime());
    expect_disabled(runtime.nostr_identity_reset_host_custody_runtime());
}

#[test]
fn nostr_disabled_paths_are_exercised() {
    let runtime = RadrootsRuntime::new().expect("runtime");

    let status = runtime.nostr_connection_status();
    assert_eq!(status.connected, 0);
    assert_eq!(status.connecting, 0);
    assert!(status.last_error.is_none());

    assert!(runtime.nostr_profile_for_self().is_none());
    assert!(runtime.nostr_next_post_event().is_none());

    expect_disabled(runtime.nostr_set_default_relays(vec!["wss://relay.example.com".to_string()]));
    expect_disabled(runtime.nostr_connect_if_key_present());
    expect_disabled(runtime.nostr_post_profile(None, None, None, None));
    expect_disabled(runtime.nostr_post_text_note("hello".to_string()));
    expect_disabled(runtime.nostr_fetch_text_notes(25, Some(0)));
    expect_disabled(runtime.nostr_post_reply(
        "event-id".to_string(),
        "author".to_string(),
        "reply".to_string(),
        None,
    ));
    expect_disabled(runtime.nostr_start_post_event_stream(None));
    expect_disabled(runtime.nostr_stop_post_event_stream());
}

#[test]
fn runtime_builder_and_logging_paths_are_exercised() {
    let handle = RuntimeBuilder::new()
        .with_config(NetConfig::default())
        .manage_runtime(false)
        .build()
        .expect("build net handle");
    drop(handle);
    let default_handle = RuntimeBuilder::new()
        .build()
        .expect("build default net handle");
    drop(default_handle);

    let err = logging::init_logging(Some("/dev/null/file.log".to_string()), None, Some(false));
    assert!(matches!(err, Err(RadrootsAppError::Msg(_))));
    let _ = logging::init_logging(None, None, None);
    let _ = logging::init_logging(None, Some("app.log".to_string()), Some(false));
    let _ = logging::init_logging_stdout();

    assert!(logging::log_info("info".to_string()).is_ok());
    assert!(logging::log_error("error".to_string()).is_ok());
    assert!(logging::log_debug("debug".to_string()).is_ok());
}

#[test]
fn nostr_records_and_enums_are_exercised() {
    let status = NostrConnectionStatus {
        light: NostrLight::Yellow,
        connected: 1,
        connecting: 2,
        last_error: Some("err".to_string()),
    };
    let _status_debug = format!("{status:?}");
    let _status_clone = status.clone();

    let profile = NostrProfile::default();
    let _profile_debug = format!("{profile:?}");
    let _profile_clone = profile.clone();

    let profile_event = NostrProfileEventMetadata {
        id: "id".to_string(),
        author: "author".to_string(),
        published_at: 1,
        profile,
    };
    let _profile_event_debug = format!("{profile_event:?}");
    let _profile_event_clone = profile_event.clone();

    let event = NostrEvent {
        id: "event-id".to_string(),
        author: "event-author".to_string(),
        created_at: 2,
        kind: 1,
        content: "content".to_string(),
    };
    let _event_debug = format!("{event:?}");
    let _event_clone = event.clone();

    let post = NostrPost {
        content: "post".to_string(),
    };
    let _post_debug = format!("{post:?}");
    let _post_clone = post.clone();

    let post_event = NostrPostEventMetadata {
        id: "id".to_string(),
        author: "author".to_string(),
        published_at: 3,
        post,
    };
    let _post_event_debug = format!("{post_event:?}");
    let _post_event_clone = post_event.clone();

    assert!(matches!(NostrLight::Red, NostrLight::Red));
    assert!(matches!(NostrLight::Green, NostrLight::Green));
}
