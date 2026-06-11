use radroots_net_core::NetHandle;
use radroots_net_core::builder::NetBuilder;
use radroots_net_core::config::NetConfig;

use crate::RadrootsAppError;

pub struct RuntimeBuilder {
    config: NetConfig,
    manage_runtime: bool,
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            config: NetConfig::default(),
            manage_runtime: true,
        }
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
        #[cfg(feature = "rt")]
        {
            match NetBuilder::new()
                .config(self.config)
                .manage_runtime(self.manage_runtime)
                .build()
            {
                Ok(handle) => Ok(handle),
                Err(err) => Err(RadrootsAppError::Msg(format!("net build failed: {err}"))),
            }
        }

        #[cfg(not(feature = "rt"))]
        {
            let handle = NetBuilder::new()
                .config(self.config)
                .manage_runtime(self.manage_runtime)
                .build()
                .expect("net build must succeed when rt feature is disabled");
            Ok(handle)
        }
    }
}
