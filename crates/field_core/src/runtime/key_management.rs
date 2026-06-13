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

#[cfg(feature = "nostr-client")]
fn account_record(
    net: &radroots_net_core::Net,
    account_id: &RadrootsIdentityId,
) -> Result<NostrIdentityRecord, RadrootsAppError> {
    let selected_identity_id = net
        .accounts
        .default_account_id()
        .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
    let account = net
        .accounts
        .list_accounts()
        .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?
        .into_iter()
        .find(|account| &account.account_id == account_id)
        .ok_or_else(|| RadrootsAppError::Msg(format!("identity not found: {account_id}")))?;
    let is_selected = selected_identity_id
        .as_ref()
        .map(|selected| selected == &account.account_id)
        .unwrap_or(false);

    Ok(NostrIdentityRecord {
        id: account.account_id.to_string(),
        public_key_hex: account.public_identity.public_key_hex,
        public_key_npub: account.public_identity.public_key_npub,
        label: account.label,
        is_selected,
    })
}

#[cfg(feature = "nostr-client")]
fn invalidate_nostr_runtime(net: &mut radroots_net_core::Net) {
    net.set_nostr_signer(None);
    net.nostr = None;
}

#[cfg(feature = "nostr-client")]
fn identity_from_secret(secret_key: &str) -> Result<RadrootsIdentity, RadrootsAppError> {
    RadrootsIdentity::from_secret_key_str(secret_key)
        .map_err(|e| RadrootsAppError::Msg(format!("{e}")))
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
    ) -> Result<NostrIdentityRecord, RadrootsAppError> {
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
            invalidate_nostr_runtime(&mut guard);
            return account_record(&guard, &account_id);
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
    ) -> Result<NostrIdentityRecord, RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let mut guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
            let identity = identity_from_secret(secret_key.as_str())?;
            let account_id = guard
                .accounts
                .upsert_identity(&identity, label, make_selected)
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            invalidate_nostr_runtime(&mut guard);
            return account_record(&guard, &account_id);
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            let _ = (secret_key, label, make_selected);
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn nostr_identity_restore_host_secret(
        &self,
        secret_key: String,
        label: Option<String>,
        make_selected: bool,
    ) -> Result<NostrIdentityRecord, RadrootsAppError> {
        self.nostr_identity_import_secret(secret_key, label, make_selected)
    }

    pub fn nostr_identity_import_from_path(
        &self,
        path: String,
        label: Option<String>,
        make_selected: bool,
    ) -> Result<NostrIdentityRecord, RadrootsAppError> {
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
            invalidate_nostr_runtime(&mut guard);
            return account_record(&guard, &account_id);
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            let _ = (path, label, make_selected);
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
            invalidate_nostr_runtime(&mut guard);
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
            invalidate_nostr_runtime(&mut guard);
            Ok(())
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            let _ = identity_id;
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn nostr_identity_clear_runtime_state(&self) -> Result<(), RadrootsAppError> {
        #[cfg(feature = "nostr-client")]
        {
            let mut guard = match self.net.lock() {
                Ok(guard) => guard,
                Err(err) => return Err(RadrootsAppError::Msg(format!("{err}"))),
            };
            let accounts = guard
                .accounts
                .list_accounts()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            for account in accounts {
                guard
                    .accounts
                    .remove_account(&account.account_id)
                    .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            }
            guard
                .accounts
                .clear_default_account()
                .map_err(|e| RadrootsAppError::Msg(format!("{e}")))?;
            invalidate_nostr_runtime(&mut guard);
            if let Ok(mut rx_guard) = self.post_events_rx.lock() {
                *rx_guard = None;
            }
            Ok(())
        }
        #[cfg(not(feature = "nostr-client"))]
        {
            Err(RadrootsAppError::Msg("nostr disabled".into()))
        }
    }

    pub fn nostr_identity_reset_all(&self) -> Result<(), RadrootsAppError> {
        self.nostr_identity_clear_runtime_state()
    }
}
