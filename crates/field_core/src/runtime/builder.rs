use radroots_net_core::NetHandle;
use radroots_net_core::builder::NetBuilder;
use radroots_net_core::config::NetConfig;

use crate::RadrootsAppError;

pub struct RuntimeBuilder {
    config: NetConfig,
    manage_runtime: bool,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self {
            config: NetConfig::default(),
            manage_runtime: true,
        }
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(mut self, config: NetConfig) -> Self {
        self.config = config;
        self
    }

    pub fn manage_runtime(mut self, manage: bool) -> Self {
        self.manage_runtime = manage;
        self
    }

    pub fn build(self) -> Result<NetHandle, RadrootsAppError> {
        NetBuilder::new()
            .config(self.config)
            .manage_runtime(self.manage_runtime)
            .build()
            .map_err(|e| RadrootsAppError::Msg(format!("net build failed: {e}")))
    }
}
