pub mod app_info;
pub mod builder;
pub mod info;
pub mod key_management;
pub mod nostr;
#[cfg(feature = "nostr-client")]
pub mod trade_listing;

use chrono::Utc;
use radroots_net_core::{NetHandle, builder::NetBuilder};
#[cfg(feature = "nostr-client")]
use std::sync::Mutex;
use std::sync::{
    RwLock,
    atomic::{AtomicBool, Ordering},
};
#[cfg(feature = "nostr-client")]
use tokio::sync::broadcast::Receiver;
use tracing::info;

use self::{
    app_info::AppInfoPlatform,
    info::{RuntimeInfo, gather_runtime_info},
};
use crate::RadrootsAppError;

#[derive(uniffi::Object)]
pub struct RadrootsRuntime {
    pub(crate) net: NetHandle,
    pub(crate) started_unix_ms: i64,
    pub(crate) shutting_down: AtomicBool,
    pub(crate) platform_app: RwLock<Option<AppInfoPlatform>>,
    #[cfg(feature = "nostr-client")]
    pub(crate) post_events_rx:
        Mutex<Option<Receiver<radroots_events::post::RadrootsPostEventMetadata>>>,
}

#[uniffi::export]
impl RadrootsRuntime {
    #[uniffi::constructor]
    pub fn new() -> Result<Self, RadrootsAppError> {
        let cfg = radroots_net_core::config::NetConfig::default();
        let handle = NetBuilder::new()
            .config(cfg)
            .manage_runtime(true)
            .build()
            .map_err(|e| RadrootsAppError::Msg(format!("net build failed: {e}")))?;

        Ok(Self {
            net: handle,
            started_unix_ms: Utc::now().timestamp_millis(),
            shutting_down: AtomicBool::new(false),
            platform_app: RwLock::new(None),
            #[cfg(feature = "nostr-client")]
            post_events_rx: Mutex::new(None),
        })
    }

    pub fn stop(&self) {
        if self.shutting_down.swap(true, Ordering::SeqCst) {
            info!("Runtime stop already in progress or completed.");
            return;
        }
        if let Ok(mut net) = self.net.lock() {
            #[cfg(feature = "rt")]
            if let Some(_rt) = net.rt.take() {
                info!("Runtime stopped gracefully.");
            } else {
                info!("No runtime was active at stop.");
            }
            #[cfg(not(feature = "rt"))]
            info!("No managed runtime is available for this build.");
        } else {
            info!("Failed to acquire runtime lock during stop.");
        }
    }

    pub fn uptime_millis(&self) -> i64 {
        Utc::now().timestamp_millis() - self.started_unix_ms
    }

    pub fn info(&self) -> RuntimeInfo {
        gather_runtime_info(self)
    }

    pub fn info_json(&self) -> String {
        serde_json::to_string_pretty(&self.info())
            .unwrap_or_else(|e| format!(r#"{{"error":"serialize RuntimeInfo: {e}"}}"#))
    }

    pub fn set_app_info_platform(
        &self,
        platform: Option<String>,
        bundle_id: Option<String>,
        version: Option<String>,
        build_number: Option<String>,
        build_sha: Option<String>,
    ) {
        let platform_info =
            AppInfoPlatform::new(platform, bundle_id, version, build_number, build_sha);
        if let Ok(mut guard) = self.platform_app.write() {
            *guard = Some(platform_info);
        }
    }
}
