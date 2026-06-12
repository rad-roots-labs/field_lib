use super::RadrootsRuntime;
use crate::RadrootsAppError;
#[cfg(feature = "nostr-client")]
use radroots_identity::{RadrootsIdentity, RadrootsIdentityId};
#[cfg(feature = "nostr-client")]
use std::path::PathBuf;

#[derive(uniffi::Record, Debug, Clone)]
pub struct NostrIdentityRecord {
    pub id: String,
    pub public_key_hex: String,
    pub public_key_npub: String,
    pub label: Option<String>,
    pub is_selected: bool,
}

#[derive(uniffi::Record, Debug, Clone)]
pub struct NostrIdentitySnapshot {
    pub has_selected_signing_identity: bool,
    pub selected_identity_id: Option<String>,
    pub selected_npub: Option<String>,
    pub identities: Vec<NostrIdentityRecord>,
}

#[cfg_attr(not(coverage_nightly), uniffi::export)]
impl RadrootsRuntime {
    pub fn nostr_identity_has_selected_signing_identity(&self) -> bool {
        #[cfg(feature = "nostr-client")]
        {
            if let Ok(guard) = self.net.lock() {
                return guard
                    .accounts
                    .default_signing_identity()
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

    pub fn nostr_identity_selected_npub(&self) -> Option<String> {
        #[cfg(feature = "nostr-client")]
        {
            if let Ok(guard) = self.net.lock() {
                return guard
                    .accounts
                    .default_public_identity()
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

    pub fn nostr_identity_list(&self) -> Result<Vec<NostrIdentityRecord>, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
            let selected_identity_id = guard
                .accounts
                .default_account_id()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            let accounts = guard
                .accounts
                .list_accounts()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            return Ok(accounts
                .into_iter()
                .map(|account| {
                    let is_selected = selected_identity_id
                        .as_ref()
                        .map(|selected| selected == &account.account_id)
                        .unwrap_or(false);
                    NostrIdentityRecord {
                        id: account.account_id.to_string(),
                        public_key_hex: account.public_identity.public_key_hex,
                        public_key_npub: account.public_identity.public_key_npub,
                        label: account.label,
                        is_selected,
                    }
                })
                .collect());
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn nostr_identity_list_ids(&self) -> Result<Vec<String>, RadrootsAppError> {
        Ok(self
            .nostr_identity_list()?
            .into_iter()
            .map(|identity| identity.id)
            .collect())
    }

    pub fn nostr_identity_snapshot(&self) -> Result<NostrIdentitySnapshot, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
            let selected_identity_id = guard
                .accounts
                .default_account_id()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            let selected_npub = guard
                .accounts
                .default_public_identity()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?
                .map(|identity| identity.public_key_npub);
            let has_selected_signing_identity = guard
                .accounts
                .default_signing_identity()
                .ok()
                .flatten()
                .is_some();
            let identities = guard
                .accounts
                .list_accounts()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?
                .into_iter()
                .map(|account| {
                    let is_selected = selected_identity_id
                        .as_ref()
                        .map(|selected| selected == &account.account_id)
                        .unwrap_or(false);
                    NostrIdentityRecord {
                        id: account.account_id.to_string(),
                        public_key_hex: account.public_identity.public_key_hex,
                        public_key_npub: account.public_identity.public_key_npub,
                        label: account.label,
                        is_selected,
                    }
                })
                .collect();
            return Ok(NostrIdentitySnapshot {
                has_selected_signing_identity,
                selected_identity_id: selected_identity_id.map(|id| id.to_string()),
                selected_npub,
                identities,
            });
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn nostr_identity_generate(
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

    pub fn nostr_identity_import_secret(
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

    pub fn nostr_identity_import_from_path(
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

    pub fn nostr_identity_export_selected_secret_hex(
        &self,
    ) -> Result<Option<String>, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
            let Some(selected_id) = guard
                .accounts
                .default_account_id()
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

    pub fn nostr_identity_select(&self, identity_id: String) -> Result<(), RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let mut guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
            let account_id = RadrootsIdentityId::parse(identity_id.as_str())
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard
                .accounts
                .set_default_account(&account_id)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            guard.nostr = None;
            Ok(())
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            let _ = identity_id;
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn nostr_identity_remove(&self, identity_id: String) -> Result<(), RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let mut guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
            let account_id = RadrootsIdentityId::parse(identity_id.as_str())
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
            let _ = identity_id;
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }
}
