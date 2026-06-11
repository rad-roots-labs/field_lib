use super::RadrootsRuntime;
use chrono::Utc;
use radroots_net_core::net;
use serde::Serialize;

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

#[derive(Debug, Clone, Serialize, uniffi::Record)]
pub struct AppInfo {
    pub build: NetBuildInfo,
    pub started_unix_ms: i64,
    pub uptime_millis: i64,
    pub shutting_down: bool,
    pub platform: Option<super::app_info::AppInfoPlatform>,
}

#[derive(Debug, Clone, Serialize, uniffi::Record)]
pub struct RuntimeInfo {
    pub app: AppInfo,
    pub net: NetBuildInfo,
}

pub fn gather_runtime_info(runtime: &RadrootsRuntime) -> RuntimeInfo {
    let now_ms = Utc::now().timestamp_millis();
    let app_info = AppInfo {
        build: app_build_info(),
        started_unix_ms: runtime.started_unix_ms,
        uptime_millis: now_ms - runtime.started_unix_ms,
        shutting_down: runtime
            .shutting_down
            .load(std::sync::atomic::Ordering::SeqCst),
        platform: runtime.platform_app.read().ok().and_then(|g| (*g).clone()),
    };

    let net_info = match runtime.net.lock() {
        Ok(guard) => NetBuildInfo::from(&guard.info.build),
        Err(_) => NetBuildInfo::default(),
    };

    RuntimeInfo {
        app: app_info,
        net: net_info,
    }
}

pub fn app_build_info() -> NetBuildInfo {
    NetBuildInfo {
        crate_name: env!("CARGO_PKG_NAME").to_string(),
        crate_version: env!("CARGO_PKG_VERSION").to_string(),
        rustc: env_opt_to_owned(option_env!("RUSTC_VERSION")),
        profile: env_opt_to_owned(option_env!("PROFILE")),
        git_sha: env_opt_to_owned(option_env!("GIT_HASH")),
        build_time_unix: env_opt_to_u64(option_env!("BUILD_TIME_UNIX")),
    }
}

fn env_opt_to_owned(value: Option<&str>) -> Option<String> {
    value.map(str::to_owned)
}

fn env_opt_to_u64(value: Option<&str>) -> Option<u64> {
    value.map(str::parse::<u64>).and_then(Result::ok)
}

#[cfg(test)]
mod tests {
    use super::NetBuildInfo;
    use radroots_net_core::net;

    #[test]
    fn net_build_info_from_copies_optional_fields() {
        let source = net::BuildInfo {
            crate_name: "radroots-net-core",
            crate_version: "1.2.3",
            rustc: Some("rustc 1.92.0"),
            profile: Some("debug"),
            git_sha: Some("abc123"),
            build_time_unix: Some(1_700_000_000),
        };

        let out = NetBuildInfo::from(&source);
        assert_eq!(out.crate_name, "radroots-net-core");
        assert_eq!(out.crate_version, "1.2.3");
        assert_eq!(out.rustc.as_deref(), Some("rustc 1.92.0"));
        assert_eq!(out.profile.as_deref(), Some("debug"));
        assert_eq!(out.git_sha.as_deref(), Some("abc123"));
        assert_eq!(out.build_time_unix, Some(1_700_000_000));
    }

    #[test]
    fn env_opt_helpers_cover_some_none_and_parse_failure() {
        assert_eq!(super::env_opt_to_owned(Some("abc")).as_deref(), Some("abc"));
        assert_eq!(super::env_opt_to_owned(None), None);
        assert_eq!(super::env_opt_to_u64(Some("123")), Some(123));
        assert_eq!(super::env_opt_to_u64(Some("abc")), None);
        assert_eq!(super::env_opt_to_u64(None), None);
    }
}
