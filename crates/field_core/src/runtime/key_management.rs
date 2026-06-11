use super::RadrootsRuntime;
use crate::RadrootsAppError;
#[cfg(feature = "nostr-client")]
use radroots_identity::{RadrootsIdentity, RadrootsIdentityId};
#[cfg(feature = "nostr-client")]
use std::path::PathBuf;

#[cfg_attr(not(coverage_nightly), uniffi::export)]
impl RadrootsRuntime {
    pub fn accounts_has_selected_signing_identity(&self) -> bool {
        #[cfg(feature = "nostr-client")]
        {
            if let Ok(guard) = self.net.lock() {
                return guard
                    .accounts
                    .selected_signing_identity()
                    .ok()
                    .flatten()
                    .is_some();
            }
        }

        #[cfg(not(feature = "nostr-client"))]
        {
            false
        }

        #[cfg(feature = "nostr-client")]
        false
    }

    pub fn accounts_selected_npub(&self) -> Option<String> {
        #[cfg(feature = "nostr-client")]
        {
            if let Ok(guard) = self.net.lock() {
                return guard
                    .accounts
                    .selected_public_identity()
                    .ok()
                    .flatten()
                    .map(|identity| identity.public_key_npub);
            }
        }

        #[cfg(not(feature = "nostr-client"))]
        {
            None
        }

        #[cfg(feature = "nostr-client")]
        None
    }

    pub fn accounts_list_ids(&self) -> Result<Vec<String>, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
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
        #[cfg(feature = "nostr-client")]
        {
            let mut guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
            let account_id = guard
                .accounts
                .generate_identity(label, make_selected)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard.nostr = None;
            return Ok(account_id.to_string());
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            let _ = (label, make_selected);
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn accounts_import_secret(
        &self,
        secret_key: String,
        label: Option<String>,
        make_selected: bool,
    ) -> Result<String, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let mut guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
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
            let _ = (secret_key, label, make_selected);
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn accounts_import_from_path(
        &self,
        path: String,
        label: Option<String>,
        make_selected: bool,
    ) -> Result<String, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let mut guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
            let account_id = guard
                .accounts
                .migrate_legacy_identity_file(PathBuf::from(path), label, make_selected)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard.nostr = None;
            return Ok(account_id.to_string());
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            let _ = (path, label, make_selected);
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn accounts_export_selected_secret_hex(&self) -> Result<Option<String>, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
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
        #[cfg(feature = "nostr-client")]
        {
            let mut guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
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
            let _ = account_id;
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn accounts_remove(&self, account_id: String) -> Result<(), RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let mut guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
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
            let _ = account_id;
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }
}
