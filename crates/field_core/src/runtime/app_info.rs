#[derive(Debug, Clone, Default, serde::Serialize, uniffi::Record)]
pub struct AppInfoPlatform {
    pub platform: Option<String>,
    pub bundle_id: Option<String>,
    pub version: Option<String>,
    pub build_number: Option<String>,
    pub build_sha: Option<String>,
}

impl AppInfoPlatform {
    pub fn new(
        platform: Option<String>,
        bundle_id: Option<String>,
        version: Option<String>,
        build_number: Option<String>,
        build_sha: Option<String>,
    ) -> Self {
        Self {
            platform,
            bundle_id,
            version,
            build_number,
            build_sha,
        }
    }
}
