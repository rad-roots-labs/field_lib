use super::RadrootsRuntime;
use crate::RadrootsAppError;
#[cfg(feature = "nostr-client")]
use radroots_identity::{RadrootsIdentity, RadrootsIdentityId};
#[cfg(feature = "nostr-client")]
use std::path::PathBuf;

#[uniffi::export]
impl RadrootsRuntime {
    pub fn accounts_has_selected_signing_identity(&self) -> bool {
        if let Ok(guard) = self.net.lock() {
            #[cfg(feature = "nostr-client")]
            {
                return guard
                    .accounts
                    .selected_signing_identity()
                    .ok()
                    .flatten()
                    .is_some();
            }
            #[cfg(not(feature = "nostr-client"))]
            {
                return false;
            }
        }
        false
    }

    pub fn accounts_selected_npub(&self) -> Option<String> {
        if let Ok(guard) = self.net.lock() {
            #[cfg(feature = "nostr-client")]
            {
                return guard
                    .accounts
                    .selected_public_identity()
                    .ok()
                    .flatten()
                    .map(|identity| identity.public_key_npub);
            }
        }
        None
    }

    pub fn accounts_list_ids(&self) -> Result<Vec<String>, RadrootsAppError> {
        let guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let accounts = guard
                .accounts
                .list_accounts()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            return Ok(accounts
                .into_iter()
                .map(|account| account.account_id.to_string())
                .collect());
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn accounts_generate(
        &self,
        label: Option<String>,
        make_selected: bool,
    ) -> Result<String, RadrootsAppError> {
        let mut guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let account_id = guard
                .accounts
                .generate_identity(label, make_selected)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard.nostr = None;
            return Ok(account_id.to_string());
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn accounts_import_secret(
        &self,
        secret_key: String,
        label: Option<String>,
        make_selected: bool,
    ) -> Result<String, RadrootsAppError> {
        let mut guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let identity = RadrootsIdentity::from_secret_key_str(secret_key.as_str())
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            let account_id = guard
                .accounts
                .upsert_identity(&identity, label, make_selected)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard.nostr = None;
            return Ok(account_id.to_string());
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn accounts_import_from_path(
        &self,
        path: String,
        label: Option<String>,
        make_selected: bool,
    ) -> Result<String, RadrootsAppError> {
        let mut guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let account_id = guard
                .accounts
                .migrate_legacy_identity_file(PathBuf::from(path), label, make_selected)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard.nostr = None;
            return Ok(account_id.to_string());
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn accounts_export_selected_secret_hex(&self) -> Result<Option<String>, RadrootsAppError> {
        let guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let Some(selected_id) = guard
                .accounts
                .selected_account_id()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?
            else {
                return Ok(None);
            };
            return guard
                .accounts
                .export_secret_hex(&selected_id)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")));
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn accounts_select(&self, account_id: String) -> Result<(), RadrootsAppError> {
        let mut guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let account_id = RadrootsIdentityId::parse(account_id.as_str())
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard
                .accounts
                .select_account(&account_id)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard.nostr = None;
            Ok(())
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn accounts_remove(&self, account_id: String) -> Result<(), RadrootsAppError> {
        let mut guard = self
            .net
            .lock()
            .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
        #[cfg(feature = "nostr-client")]
        {
            let account_id = RadrootsIdentityId::parse(account_id.as_str())
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard
                .accounts
                .remove_account(&account_id)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard.nostr = None;
            Ok(())
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }
}
