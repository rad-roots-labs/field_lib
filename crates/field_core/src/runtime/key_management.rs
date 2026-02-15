use super::RadrootsRuntime;
use crate::RadrootsAppError;
use std::path::PathBuf;

#[uniffi::export]
impl RadrootsRuntime {
    pub fn keys_is_loaded(&self) -> bool {
        if let Ok(guard) = self.net.lock() {
            #[cfg(feature = "nostr-client")]
            {
                return guard.keys.state.loaded;
            }
            #[cfg(not(feature = "nostr-client"))]
            {
                return false;
            }
        }
        false
    }

    pub fn keys_npub(&self) -> Option<String> {
        if let Ok(guard) = self.net.lock() {
            #[cfg(feature = "nostr-client")]
            {
                return guard.keys.npub();
            }
        }
        None
    }

    pub fn keys_generate_in_memory(&self) -> Result<String, RadrootsAppError> {
        let mut guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let k = guard.keys.generate_in_memory();
            return Ok(k.public_key().to_string());
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn keys_export_secret_hex(&self) -> Result<String, RadrootsAppError> {
        let guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            return guard
                .keys
                .export_secret_hex()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")));
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn keys_load_hex32(&self, hex: String) -> Result<(), RadrootsAppError> {
        let mut guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            guard
                .keys
                .load_from_hex32(&hex)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            Ok(())
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn keys_load_from_path_auto(&self, path: String) -> Result<(), RadrootsAppError> {
        let mut guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            guard
                .keys
                .load_from_path_auto(PathBuf::from(path))
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            Ok(())
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn keys_persist_best_practice(&self) -> Result<String, RadrootsAppError> {
        let _guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(all(
            feature = "nostr-client",
            feature = "directories",
            feature = "fs-persistence"
        ))]
        {
            let p = _guard
                .keys
                .persist_best_practice()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            Ok(p.display().to_string())
        }
        #[cfg(not(all(
            feature = "nostr-client",
            feature = "directories",
            feature = "fs-persistence"
        )))]
        {
            Err(RadrootsAppError::Msg("persistence unsupported".into()))
        }
    }
}
