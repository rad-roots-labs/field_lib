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

#[cfg_attr(not(coverage_nightly), uniffi::export)]
impl RadrootsRuntime {
    #[cfg_attr(not(coverage_nightly), uniffi::constructor)]
    pub fn new() -> Result<Self, RadrootsAppError> {
        let cfg = radroots_net_core::config::NetConfig::default();
        #[cfg(feature = "rt")]
        let handle = match NetBuilder::new().config(cfg).manage_runtime(true).build() {
            Ok(handle) => handle,
            Err(err) => return Err(RadrootsAppError::Msg(format!("net build failed: {err}"))),
        };
        #[cfg(not(feature = "rt"))]
        let handle = NetBuilder::new()
            .config(cfg)
            .manage_runtime(true)
            .build()
            .expect("net build must succeed when rt feature is disabled");

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

        #[cfg(feature = "rt")]
        {
            if let Ok(mut net) = self.net.lock() {
                if let Some(_rt) = net.rt.take() {
                    info!("Runtime stopped gracefully.");
                } else {
                    info!("No runtime was active at stop.");
                }
            } else {
                info!("Failed to acquire runtime lock during stop.");
            }
        }

        #[cfg(not(feature = "rt"))]
        {
            info!("No managed runtime is available for this build.");
        }
    }

    pub fn uptime_millis(&self) -> i64 {
        Utc::now().timestamp_millis() - self.started_unix_ms
    }

    pub fn info(&self) -> RuntimeInfo {
        gather_runtime_info(self)
    }

    pub fn info_json(&self) -> String {
        #[cfg(feature = "rt")]
        {
            return match serde_json::to_string_pretty(&self.info()) {
                Ok(json) => json,
                Err(err) => format!(r#"{{"error":"serialize RuntimeInfo: {err}"}}"#),
            };
        }
        #[cfg(not(feature = "rt"))]
        {
            serde_json::to_string_pretty(&self.info())
                .expect("runtime info serialization must succeed in no-rt builds")
        }
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

#[cfg(test)]
mod tests {
    use super::RadrootsRuntime;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn poison_net_lock(runtime: &RadrootsRuntime) {
        let handle = runtime.net.clone();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = handle.lock().expect("lock net");
            panic!("poison net lock");
        }));
    }

    fn poison_platform_lock(runtime: &RadrootsRuntime) {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = runtime.platform_app.write().expect("lock platform");
            panic!("poison platform lock");
        }));
    }

    #[test]
    fn runtime_info_uses_default_net_info_when_lock_is_poisoned() {
        let runtime = RadrootsRuntime::new().expect("runtime");
        poison_net_lock(&runtime);

        let _ = runtime.uptime_millis();
        let info = runtime.info();
        assert_eq!(info.net.crate_name, String::new());
        assert_eq!(info.net.crate_version, String::new());
        let json = runtime.info_json();
        assert!(json.contains("\"net\""));
        runtime.stop();
        runtime.stop();
    }

    #[test]
    fn set_platform_info_handles_poisoned_lock() {
        let runtime = RadrootsRuntime::new().expect("runtime");
        poison_platform_lock(&runtime);
        runtime.set_app_info_platform(
            Some("ios".to_string()),
            Some("org.radroots.app".to_string()),
            Some("1.0.0".to_string()),
            Some("100".to_string()),
            Some("abc123".to_string()),
        );
    }
}
