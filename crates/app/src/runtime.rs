use chrono::Utc;
use radroots_net_core::{NetHandle, builder::NetBuilder, net};
use serde::Serialize;
use std::sync::{
    RwLock,
    atomic::{AtomicBool, Ordering},
};
use tracing::info;

#[inline]
fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Serialize, Default, uniffi::Record)]
pub struct NetBuildInfo {
    pub crate_name: String,
    pub crate_version: String,
    pub rustc: Option<String>,
    pub profile: Option<String>,
    pub git_sha: Option<String>,
    pub build_time_unix: Option<u64>,
}

impl From<&net::BuildInfo> for NetBuildInfo {
    fn from(b: &net::BuildInfo) -> Self {
        Self {
            crate_name: b.crate_name.to_string(),
            crate_version: b.crate_version.to_string(),
            rustc: b.rustc.map(|s| s.to_string()),
            profile: b.profile.map(|s| s.to_string()),
            git_sha: b.git_sha.map(|s| s.to_string()),
            build_time_unix: b.build_time_unix,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default, uniffi::Record)]
pub struct AppInfoPlatform {
    pub platform: Option<String>,
    pub bundle_id: Option<String>,
    pub version: Option<String>,
    pub build_number: Option<String>,
    pub build_sha: Option<String>,
}

pub type AppBuildInfo = NetBuildInfo;

#[derive(Debug, Clone, Serialize, uniffi::Record)]
pub struct AppInfo {
    pub build: AppBuildInfo,
    pub platform: Option<AppInfoPlatform>,
    pub started_unix_ms: i64,
    pub uptime_millis: i64,
    #[serde(skip_serializing_if = "is_false")]
    pub shutting_down: bool,
}

#[derive(Debug, Clone, Serialize, uniffi::Record)]
pub struct RuntimeInfo {
    pub app: AppInfo,
    pub net: NetBuildInfo,
}

#[derive(uniffi::Object)]
pub struct RadrootsRuntime {
    net: NetHandle,
    started_unix_ms: i64,
    shutting_down: AtomicBool,
    platform_app: RwLock<Option<AppInfoPlatform>>,
}

impl RadrootsRuntime {
    fn app_build_info() -> NetBuildInfo {
        NetBuildInfo {
            crate_name: env!("CARGO_PKG_NAME").to_string(),
            crate_version: env!("CARGO_PKG_VERSION").to_string(),
            rustc: option_env!("RUSTC_VERSION").map(|s| s.to_string()),
            profile: option_env!("PROFILE").map(|s| s.to_string()),
            git_sha: option_env!("GIT_HASH").map(|s| s.to_string()),
            build_time_unix: option_env!("BUILD_TIME_UNIX").and_then(|s| s.parse().ok()),
        }
    }

    fn uptime_millis_from(&self, now_ms: i64) -> i64 {
        now_ms.saturating_sub(self.started_unix_ms)
    }
}

#[uniffi::export]
impl RadrootsRuntime {
    #[uniffi::constructor]
    pub fn new() -> Result<Self, crate::RadrootsAppError> {
        let cfg = radroots_net_core::config::NetConfig::default();
        let handle = NetBuilder::new()
            .config(cfg)
            .manage_runtime(true)
            .build()
            .map_err(|e| crate::RadrootsAppError::Msg(format!("net build failed: {e}")))?;

        Ok(Self {
            net: handle,
            started_unix_ms: Utc::now().timestamp_millis(),
            shutting_down: AtomicBool::new(false),
            platform_app: RwLock::new(None),
        })
    }

    pub fn set_app_info_platform(
        &self,
        platform: Option<String>,
        bundle_id: Option<String>,
        version: Option<String>,
        build_number: Option<String>,
        build_sha: Option<String>,
    ) {
        let info = AppInfoPlatform {
            platform,
            bundle_id,
            version,
            build_number,
            build_sha,
        };
        if let Ok(mut guard) = self.platform_app.write() {
            *guard = Some(info);
        }
    }

    pub fn stop(&self) {
        if self
            .shutting_down
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            info!("Runtime stop already in progress or completed.");
            return;
        }

        if let Ok(mut net) = self.net.lock() {
            if let Some(_rt) = net.rt.take() {
                info!("The runtime stopped gracefully.");
            } else {
                info!("No runtime was active at stop call.");
            }
        } else {
            info!("Failed to acquire runtime lock during stop.");
        }
    }

    pub fn uptime_millis(&self) -> i64 {
        self.uptime_millis_from(Utc::now().timestamp_millis())
    }

    pub fn info(&self) -> RuntimeInfo {
        let now_ms = Utc::now().timestamp_millis();
        let app = AppInfo {
            build: Self::app_build_info(),
            platform: self.platform_app.read().ok().and_then(|g| (*g).clone()),
            started_unix_ms: self.started_unix_ms,
            uptime_millis: self.uptime_millis_from(now_ms),
            shutting_down: self.shutting_down.load(Ordering::SeqCst),
        };
        let net = match self.net.lock() {
            Ok(guard) => NetBuildInfo::from(&guard.info.build),
            Err(_) => NetBuildInfo::default(),
        };
        RuntimeInfo { app, net }
    }

    pub fn info_json(&self) -> String {
        match serde_json::to_string_pretty(&self.info()) {
            Ok(s) => s,
            Err(e) => format!(r#"{{"error":"failed to serialize RuntimeInfo: {e}"}}"#),
        }
    }
}
