use std::time::Duration;

use reqwest::{Url, blocking::Client};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::{
    RadrootsRuntime,
    nostr::{NostrConnectionStatus, NostrLight},
};
use crate::RadrootsAppError;

const AUTH_HTTP_TIMEOUT_SECONDS: u64 = 15;

#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSessionPhase {
    SignedOut,
    ChallengePending,
    Authenticated,
    Expired,
    Revoked,
}

impl Default for FieldSessionPhase {
    fn default() -> Self {
        Self::SignedOut
    }
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldAuthConfig {
    pub auth_api_base_url: Option<String>,
    pub accounts_api_base_url: Option<String>,
}

impl FieldAuthConfig {
    fn from_inputs(
        auth_api_base_url: Option<String>,
        accounts_api_base_url: Option<String>,
    ) -> Result<Self, RadrootsAppError> {
        Ok(Self {
            auth_api_base_url: normalize_http_base_url(auth_api_base_url, "auth_api_base_url")?,
            accounts_api_base_url: normalize_http_base_url(
                accounts_api_base_url,
                "accounts_api_base_url",
            )?,
        })
    }
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct FieldLoginChallenge {
    pub id: String,
    pub challenge_kind: String,
    pub login_username: Option<String>,
    pub masked_email: String,
    pub delivery_state: String,
    pub max_attempts: i32,
    pub attempt_count: i32,
    pub expires_at_unix_seconds: i64,
    pub delivered_at_unix_seconds: Option<i64>,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct FieldSessionAccount {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub status: String,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct FieldSessionProfile {
    pub id: String,
    pub account_id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub status: String,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct FieldSessionCredential {
    pub id: String,
    pub account_id: String,
    pub profile_id: String,
    pub email: String,
    pub status: String,
    pub is_primary: bool,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct FieldSession {
    pub id: String,
    pub account_id: String,
    pub profile_id: String,
    pub credential_id: String,
    pub session_id: String,
    pub status: String,
    pub expires_at_unix_seconds: i64,
    pub revoked_at_unix_seconds: Option<i64>,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct FieldSessionSnapshot {
    pub phase: FieldSessionPhase,
    pub pending_challenge: Option<FieldLoginChallenge>,
    pub account: Option<FieldSessionAccount>,
    pub profile: Option<FieldSessionProfile>,
    pub credential: Option<FieldSessionCredential>,
    pub session: Option<FieldSession>,
    pub access_token_present: bool,
    pub refresh_token_present: bool,
    pub selected_npub: Option<String>,
    pub nostr_light: NostrLight,
    pub nostr_connected: u32,
    pub nostr_connecting: u32,
    pub nostr_last_error: Option<String>,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct FieldSessionTokenBundle {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FieldSessionState {
    phase: FieldSessionPhase,
    pending_challenge: Option<FieldLoginChallenge>,
    account: Option<FieldSessionAccount>,
    profile: Option<FieldSessionProfile>,
    credential: Option<FieldSessionCredential>,
    session: Option<FieldSession>,
    access_token: Option<String>,
    refresh_token: Option<String>,
}

impl FieldSessionState {
    fn challenge_pending(challenge: FieldLoginChallenge) -> Self {
        Self {
            phase: FieldSessionPhase::ChallengePending,
            pending_challenge: Some(challenge),
            ..Self::default()
        }
    }

    fn authenticated(bundle: SessionBundleResponseDto) -> Self {
        Self {
            phase: FieldSessionPhase::Authenticated,
            pending_challenge: None,
            account: Some(bundle.account.into()),
            profile: Some(bundle.profile.into()),
            credential: Some(bundle.credential.into()),
            session: Some(bundle.session.into()),
            access_token: Some(bundle.access_token),
            refresh_token: Some(bundle.refresh_token),
        }
    }

    fn restored(
        view: SessionResponseDto,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Self {
        Self {
            phase: FieldSessionPhase::Authenticated,
            pending_challenge: None,
            account: Some(view.account.into()),
            profile: Some(view.profile.into()),
            credential: Some(view.credential.into()),
            session: Some(view.session.into()),
            access_token: Some(access_token),
            refresh_token,
        }
    }

    fn revoked(view: SessionResponseDto) -> Self {
        Self {
            phase: FieldSessionPhase::Revoked,
            pending_challenge: None,
            account: Some(view.account.into()),
            profile: Some(view.profile.into()),
            credential: Some(view.credential.into()),
            session: Some(view.session.into()),
            access_token: None,
            refresh_token: None,
        }
    }

    fn snapshot(
        &self,
        selected_npub: Option<String>,
        nostr: NostrConnectionStatus,
    ) -> FieldSessionSnapshot {
        FieldSessionSnapshot {
            phase: self.phase,
            pending_challenge: self.pending_challenge.clone(),
            account: self.account.clone(),
            profile: self.profile.clone(),
            credential: self.credential.clone(),
            session: self.session.clone(),
            access_token_present: self.access_token.is_some(),
            refresh_token_present: self.refresh_token.is_some(),
            selected_npub,
            nostr_light: nostr.light,
            nostr_connected: nostr.connected,
            nostr_connecting: nostr.connecting,
            nostr_last_error: nostr.last_error,
        }
    }

    fn token_bundle(&self) -> Option<FieldSessionTokenBundle> {
        Some(FieldSessionTokenBundle {
            access_token: self.access_token.clone()?,
            refresh_token: self.refresh_token.clone()?,
        })
    }
}

#[cfg_attr(not(coverage_nightly), uniffi::export)]
impl RadrootsRuntime {
    pub fn field_configure_auth(
        &self,
        auth_api_base_url: Option<String>,
        accounts_api_base_url: Option<String>,
    ) -> Result<(), RadrootsAppError> {
        let config = FieldAuthConfig::from_inputs(auth_api_base_url, accounts_api_base_url)?;
        let mut guard = self
            .auth_config
            .write()
            .map_err(|err| RadrootsAppError::Msg(format!("{err}")))?;
        *guard = config;
        Ok(())
    }

    pub fn field_auth_config(&self) -> FieldAuthConfig {
        self.auth_config
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn field_session_snapshot(&self) -> FieldSessionSnapshot {
        self.snapshot_from_state()
    }

    pub fn field_session_token_bundle(
        &self,
    ) -> Result<Option<FieldSessionTokenBundle>, RadrootsAppError> {
        let guard = self
            .session
            .read()
            .map_err(|err| RadrootsAppError::Msg(format!("{err}")))?;
        Ok(guard.token_bundle())
    }

    pub fn field_start_login(
        &self,
        username: String,
    ) -> Result<FieldLoginChallenge, RadrootsAppError> {
        let username = non_empty(username, "username")?;
        let response: ChallengeResponseDto =
            self.auth_post_public("/v1/auth/login", &StartLoginRequest { username })?;
        let challenge: FieldLoginChallenge = response.challenge.into();
        self.replace_session(FieldSessionState::challenge_pending(challenge.clone()))?;
        Ok(challenge)
    }

    pub fn field_get_login_challenge(
        &self,
        challenge_id: String,
    ) -> Result<FieldLoginChallenge, RadrootsAppError> {
        let challenge_id = path_id(challenge_id, "challenge_id")?;
        let response: ChallengeResponseDto =
            self.auth_get_public(format!("/v1/auth/challenges/{challenge_id}").as_str())?;
        let challenge: FieldLoginChallenge = response.challenge.into();
        self.replace_session(FieldSessionState::challenge_pending(challenge.clone()))?;
        Ok(challenge)
    }

    pub fn field_resend_login_challenge(
        &self,
        challenge_id: String,
    ) -> Result<FieldLoginChallenge, RadrootsAppError> {
        let challenge_id = path_id(challenge_id, "challenge_id")?;
        let response: ChallengeResponseDto = self.auth_post_public(
            format!("/v1/auth/challenges/{challenge_id}/resend").as_str(),
            &EmptyRequest {},
        )?;
        let challenge: FieldLoginChallenge = response.challenge.into();
        self.replace_session(FieldSessionState::challenge_pending(challenge.clone()))?;
        Ok(challenge)
    }

    pub fn field_verify_login_challenge(
        &self,
        challenge_id: String,
        code: String,
    ) -> Result<FieldSessionSnapshot, RadrootsAppError> {
        let challenge_id = path_id(challenge_id, "challenge_id")?;
        let code = non_empty(code, "code")?;
        let response: SessionBundleResponseDto = self.auth_post_public(
            format!("/v1/auth/challenges/{challenge_id}/verify").as_str(),
            &VerifyChallengeRequest { code },
        )?;
        self.replace_session(FieldSessionState::authenticated(response))?;
        Ok(self.snapshot_from_state())
    }

    pub fn field_refresh_session(
        &self,
        request_id: String,
    ) -> Result<FieldSessionSnapshot, RadrootsAppError> {
        let request_id = non_empty(request_id, "request_id")?;
        let refresh_token = self.require_refresh_token()?;
        let response: SessionBundleResponseDto = self.auth_post_public(
            "/v1/auth/session/refresh",
            &RefreshSessionRequest {
                request_id,
                refresh_token,
            },
        )?;
        self.replace_session(FieldSessionState::authenticated(response))?;
        Ok(self.snapshot_from_state())
    }

    pub fn field_restore_session(
        &self,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Result<FieldSessionSnapshot, RadrootsAppError> {
        let access_token = non_empty(access_token, "access_token")?;
        let refresh_token = optional_non_empty(refresh_token, "refresh_token")?;
        let response: SessionResponseDto =
            self.auth_get_bearer("/v1/auth/session", &access_token)?;
        self.replace_session(FieldSessionState::restored(
            response,
            access_token,
            refresh_token,
        ))?;
        Ok(self.snapshot_from_state())
    }

    pub fn field_current_session(&self) -> Result<FieldSessionSnapshot, RadrootsAppError> {
        let access_token = self.require_access_token()?;
        let response: SessionResponseDto =
            self.auth_get_bearer("/v1/auth/session", &access_token)?;
        let refresh_token = self.optional_refresh_token()?;
        self.replace_session(FieldSessionState::restored(
            response,
            access_token,
            refresh_token,
        ))?;
        Ok(self.snapshot_from_state())
    }

    pub fn field_revoke_session(&self) -> Result<FieldSessionSnapshot, RadrootsAppError> {
        let access_token = self.require_access_token()?;
        let session_id = self.require_session_id()?;
        let response: SessionResponseDto = self.auth_post_bearer(
            "/v1/auth/session/revoke",
            &access_token,
            &RevokeSessionRequest { session_id },
        )?;
        self.replace_session(FieldSessionState::revoked(response))?;
        Ok(self.snapshot_from_state())
    }

    pub fn field_clear_session(&self) -> FieldSessionSnapshot {
        let _ = self.replace_session(FieldSessionState::default());
        self.snapshot_from_state()
    }

    pub fn field_prepare_authenticated_nostr(
        &self,
        relays: Vec<String>,
    ) -> Result<FieldSessionSnapshot, RadrootsAppError> {
        self.require_authenticated_session()?;
        if relays.is_empty() {
            return Err(RadrootsAppError::Msg(
                "at least one relay is required".into(),
            ));
        }
        if !self.accounts_has_selected_signing_identity() {
            let label = self
                .session
                .read()
                .ok()
                .and_then(|guard| {
                    guard
                        .account
                        .as_ref()
                        .map(|account| account.username.clone())
                })
                .unwrap_or_else(|| "Radroots Field".to_owned());
            self.accounts_generate(Some(label), true)?;
        }
        self.nostr_set_default_relays(relays)?;
        self.nostr_connect_if_key_present()?;
        Ok(self.snapshot_from_state())
    }
}

impl RadrootsRuntime {
    fn snapshot_from_state(&self) -> FieldSessionSnapshot {
        let selected_npub = self.accounts_selected_npub();
        let nostr = self.nostr_connection_status();
        self.session
            .read()
            .map(|guard| guard.snapshot(selected_npub, nostr))
            .unwrap_or_else(|_| {
                FieldSessionState::default().snapshot(
                    self.accounts_selected_npub(),
                    NostrConnectionStatus {
                        light: NostrLight::Red,
                        connected: 0,
                        connecting: 0,
                        last_error: None,
                    },
                )
            })
    }

    fn replace_session(&self, session: FieldSessionState) -> Result<(), RadrootsAppError> {
        let mut guard = self
            .session
            .write()
            .map_err(|err| RadrootsAppError::Msg(format!("{err}")))?;
        *guard = session;
        Ok(())
    }

    fn require_authenticated_session(&self) -> Result<(), RadrootsAppError> {
        let guard = self
            .session
            .read()
            .map_err(|err| RadrootsAppError::Msg(format!("{err}")))?;
        if guard.phase == FieldSessionPhase::Authenticated && guard.access_token.is_some() {
            Ok(())
        } else {
            Err(RadrootsAppError::Msg(
                "authenticated field session is required".into(),
            ))
        }
    }

    fn require_access_token(&self) -> Result<String, RadrootsAppError> {
        let guard = self
            .session
            .read()
            .map_err(|err| RadrootsAppError::Msg(format!("{err}")))?;
        guard
            .access_token
            .clone()
            .ok_or_else(|| RadrootsAppError::Msg("access token is not configured".into()))
    }

    fn optional_refresh_token(&self) -> Result<Option<String>, RadrootsAppError> {
        let guard = self
            .session
            .read()
            .map_err(|err| RadrootsAppError::Msg(format!("{err}")))?;
        Ok(guard.refresh_token.clone())
    }

    fn require_refresh_token(&self) -> Result<String, RadrootsAppError> {
        let guard = self
            .session
            .read()
            .map_err(|err| RadrootsAppError::Msg(format!("{err}")))?;
        guard
            .refresh_token
            .clone()
            .ok_or_else(|| RadrootsAppError::Msg("refresh token is not configured".into()))
    }

    fn require_session_id(&self) -> Result<String, RadrootsAppError> {
        let guard = self
            .session
            .read()
            .map_err(|err| RadrootsAppError::Msg(format!("{err}")))?;
        guard
            .session
            .as_ref()
            .map(|session| session.session_id.clone())
            .ok_or_else(|| RadrootsAppError::Msg("session id is not configured".into()))
    }

    fn auth_base_url(&self) -> Result<String, RadrootsAppError> {
        let guard = self
            .auth_config
            .read()
            .map_err(|err| RadrootsAppError::Msg(format!("{err}")))?;
        guard
            .auth_api_base_url
            .clone()
            .ok_or_else(|| RadrootsAppError::Msg("auth API base URL is not configured".into()))
    }

    fn auth_url(&self, path: &str) -> Result<Url, RadrootsAppError> {
        if !path.starts_with('/') {
            return Err(RadrootsAppError::Msg(
                "auth API path must start with /".into(),
            ));
        }
        Url::parse(format!("{}{}", self.auth_base_url()?, path).as_str())
            .map_err(|err| RadrootsAppError::Msg(format!("auth API URL is invalid: {err}")))
    }

    fn auth_get_public<T>(&self, path: &str) -> Result<T, RadrootsAppError>
    where
        T: DeserializeOwned,
    {
        self.auth_get(path, None)
    }

    fn auth_get_bearer<T>(&self, path: &str, access_token: &str) -> Result<T, RadrootsAppError>
    where
        T: DeserializeOwned,
    {
        self.auth_get(path, Some(access_token))
    }

    fn auth_get<T>(&self, path: &str, access_token: Option<&str>) -> Result<T, RadrootsAppError>
    where
        T: DeserializeOwned,
    {
        let client = auth_client()?;
        let mut request = client.get(self.auth_url(path)?);
        if let Some(token) = access_token {
            request = request.bearer_auth(token);
        }
        decode_response("GET", path, request.send())
    }

    fn auth_post_public<B, T>(&self, path: &str, body: &B) -> Result<T, RadrootsAppError>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        self.auth_post(path, None, body)
    }

    fn auth_post_bearer<B, T>(
        &self,
        path: &str,
        access_token: &str,
        body: &B,
    ) -> Result<T, RadrootsAppError>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        self.auth_post(path, Some(access_token), body)
    }

    fn auth_post<B, T>(
        &self,
        path: &str,
        access_token: Option<&str>,
        body: &B,
    ) -> Result<T, RadrootsAppError>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let client = auth_client()?;
        let mut request = client.post(self.auth_url(path)?).json(body);
        if let Some(token) = access_token {
            request = request.bearer_auth(token);
        }
        decode_response("POST", path, request.send())
    }
}

#[derive(Debug, Serialize)]
struct EmptyRequest {}

#[derive(Debug, Serialize)]
struct StartLoginRequest {
    username: String,
}

#[derive(Debug, Serialize)]
struct VerifyChallengeRequest {
    code: String,
}

#[derive(Debug, Serialize)]
struct RefreshSessionRequest {
    request_id: String,
    refresh_token: String,
}

#[derive(Debug, Serialize)]
struct RevokeSessionRequest {
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct ChallengeResponseDto {
    challenge: ChallengeBodyDto,
}

#[derive(Debug, Deserialize)]
struct ChallengeBodyDto {
    id: String,
    challenge_kind: String,
    login_username: Option<String>,
    masked_email: String,
    delivery_state: String,
    max_attempts: i32,
    attempt_count: i32,
    expires_at_unix_seconds: i64,
    delivered_at_unix_seconds: Option<i64>,
}

impl From<ChallengeBodyDto> for FieldLoginChallenge {
    fn from(value: ChallengeBodyDto) -> Self {
        Self {
            id: value.id,
            challenge_kind: value.challenge_kind,
            login_username: value.login_username,
            masked_email: value.masked_email,
            delivery_state: value.delivery_state,
            max_attempts: value.max_attempts,
            attempt_count: value.attempt_count,
            expires_at_unix_seconds: value.expires_at_unix_seconds,
            delivered_at_unix_seconds: value.delivered_at_unix_seconds,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SessionBundleResponseDto {
    account: AccountBodyDto,
    profile: ProfileBodyDto,
    credential: CredentialBodyDto,
    session: SessionBodyDto,
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct SessionResponseDto {
    account: AccountBodyDto,
    profile: ProfileBodyDto,
    credential: CredentialBodyDto,
    session: SessionBodyDto,
}

#[derive(Debug, Deserialize)]
struct AccountBodyDto {
    id: String,
    username: String,
    display_name: String,
    status: String,
}

impl From<AccountBodyDto> for FieldSessionAccount {
    fn from(value: AccountBodyDto) -> Self {
        Self {
            id: value.id,
            username: value.username,
            display_name: value.display_name,
            status: value.status,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ProfileBodyDto {
    id: String,
    account_id: String,
    display_name: String,
    email: Option<String>,
    status: String,
}

impl From<ProfileBodyDto> for FieldSessionProfile {
    fn from(value: ProfileBodyDto) -> Self {
        Self {
            id: value.id,
            account_id: value.account_id,
            display_name: value.display_name,
            email: value.email,
            status: value.status,
        }
    }
}

#[derive(Debug, Deserialize)]
struct CredentialBodyDto {
    id: String,
    account_id: String,
    profile_id: String,
    email: String,
    status: String,
    is_primary: bool,
}

impl From<CredentialBodyDto> for FieldSessionCredential {
    fn from(value: CredentialBodyDto) -> Self {
        Self {
            id: value.id,
            account_id: value.account_id,
            profile_id: value.profile_id,
            email: value.email,
            status: value.status,
            is_primary: value.is_primary,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SessionBodyDto {
    id: String,
    account_id: String,
    profile_id: String,
    credential_id: String,
    session_id: String,
    status: String,
    expires_at_unix_seconds: i64,
    revoked_at_unix_seconds: Option<i64>,
}

impl From<SessionBodyDto> for FieldSession {
    fn from(value: SessionBodyDto) -> Self {
        Self {
            id: value.id,
            account_id: value.account_id,
            profile_id: value.profile_id,
            credential_id: value.credential_id,
            session_id: value.session_id,
            status: value.status,
            expires_at_unix_seconds: value.expires_at_unix_seconds,
            revoked_at_unix_seconds: value.revoked_at_unix_seconds,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiErrorBodyDto {
    code: String,
    message: String,
}

fn auth_client() -> Result<Client, RadrootsAppError> {
    Client::builder()
        .timeout(Duration::from_secs(AUTH_HTTP_TIMEOUT_SECONDS))
        .build()
        .map_err(|err| RadrootsAppError::Msg(format!("auth API client build failed: {err}")))
}

fn decode_response<T>(
    method: &str,
    path: &str,
    response: Result<reqwest::blocking::Response, reqwest::Error>,
) -> Result<T, RadrootsAppError>
where
    T: DeserializeOwned,
{
    let response =
        response.map_err(|err| RadrootsAppError::Msg(format!("{method} {path} failed: {err}")))?;
    let status = response.status();
    if status.is_success() {
        return response.json::<T>().map_err(|err| {
            RadrootsAppError::Msg(format!("{method} {path} response decode failed: {err}"))
        });
    }
    let body = response
        .text()
        .unwrap_or_else(|err| format!("failed to read error response: {err}"));
    let message = serde_json::from_str::<ApiErrorBodyDto>(body.as_str())
        .map(|body| format!("{} {}", body.code, body.message))
        .unwrap_or(body);
    Err(RadrootsAppError::Msg(format!(
        "{method} {path} failed with HTTP {status}: {message}"
    )))
}

fn normalize_http_base_url(
    value: Option<String>,
    field: &str,
) -> Result<Option<String>, RadrootsAppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().trim_end_matches('/').to_owned();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = Url::parse(value.as_str())
        .map_err(|err| RadrootsAppError::Msg(format!("{field} is invalid: {err}")))?;
    match parsed.scheme() {
        "http" | "https" => Ok(Some(value)),
        _ => Err(RadrootsAppError::Msg(format!(
            "{field} must use http or https"
        ))),
    }
}

fn optional_non_empty(
    value: Option<String>,
    field: &str,
) -> Result<Option<String>, RadrootsAppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    Ok(Some(non_empty(value, field)?))
}

fn non_empty(value: String, field: &str) -> Result<String, RadrootsAppError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(RadrootsAppError::Msg(format!("{field} is required")));
    }
    Ok(value)
}

fn path_id(value: String, field: &str) -> Result<String, RadrootsAppError> {
    let value = non_empty(value, field)?;
    if value.contains('/') || value.contains('?') || value.contains('#') {
        return Err(RadrootsAppError::Msg(format!(
            "{field} contains invalid path characters"
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use super::{FieldAuthConfig, FieldSessionPhase};
    use crate::runtime::RadrootsRuntime;

    #[test]
    fn auth_config_normalizes_and_validates_http_urls() {
        let config = FieldAuthConfig::from_inputs(
            Some(" http://127.0.0.1:8081/ ".to_owned()),
            Some("https://accounts.example.test/api/".to_owned()),
        )
        .expect("config");

        assert_eq!(
            config.auth_api_base_url.as_deref(),
            Some("http://127.0.0.1:8081")
        );
        assert_eq!(
            config.accounts_api_base_url.as_deref(),
            Some("https://accounts.example.test/api")
        );
        let error = FieldAuthConfig::from_inputs(Some("wss://relay.example.test".to_owned()), None)
            .expect_err("reject non-http URL");
        assert!(error.to_string().contains("auth_api_base_url"));
    }

    #[test]
    fn session_snapshot_redacts_tokens() {
        let runtime = RadrootsRuntime::new().expect("runtime");
        let snapshot = runtime
            .field_restore_session("access-token".to_owned(), Some("refresh-token".to_owned()))
            .expect_err("auth URL missing");
        assert!(snapshot.to_string().contains("auth API base URL"));

        runtime
            .replace_session(super::FieldSessionState {
                phase: FieldSessionPhase::Authenticated,
                access_token: Some("access-token".to_owned()),
                refresh_token: Some("refresh-token".to_owned()),
                ..Default::default()
            })
            .expect("session");

        let snapshot = runtime.field_session_snapshot();
        assert!(snapshot.access_token_present);
        assert!(snapshot.refresh_token_present);
        assert!(!format!("{snapshot:?}").contains("access-token"));
        assert!(!format!("{snapshot:?}").contains("refresh-token"));
        let tokens = runtime
            .field_session_token_bundle()
            .expect("token bundle")
            .expect("tokens");
        assert_eq!(tokens.access_token, "access-token");
        runtime.stop();
    }

    #[test]
    fn start_login_posts_to_auth_api_and_stores_pending_challenge() {
        let (base_url, handle) = spawn_response(
            "POST /v1/auth/login HTTP/1.1",
            Some("\"username\":\"field@radroots.test\""),
            r#"{"challenge":{"id":"challenge-1","challenge_kind":"login","login_username":"field@radroots.test","masked_email":"f***@radroots.test","delivery_state":"sent","max_attempts":6,"attempt_count":0,"expires_at_unix_seconds":1893456000,"delivered_at_unix_seconds":1893455900}}"#,
        );
        let runtime = RadrootsRuntime::new().expect("runtime");
        runtime
            .field_configure_auth(Some(base_url), None)
            .expect("configure");

        let challenge = runtime
            .field_start_login("field@radroots.test".to_owned())
            .expect("login");
        assert_eq!(challenge.id, "challenge-1");
        assert_eq!(
            runtime.field_session_snapshot().phase,
            FieldSessionPhase::ChallengePending
        );
        handle.join().expect("server");
        runtime.stop();
    }

    #[test]
    fn verify_login_stores_authenticated_session_and_redacted_snapshot() {
        let body = sample_session_bundle_json("access-token", "refresh-token");
        let (base_url, handle) = spawn_response(
            "POST /v1/auth/challenges/challenge-1/verify HTTP/1.1",
            Some("\"code\":\"123456\""),
            body.as_str(),
        );
        let runtime = RadrootsRuntime::new().expect("runtime");
        runtime
            .field_configure_auth(Some(base_url), None)
            .expect("configure");

        let snapshot = runtime
            .field_verify_login_challenge("challenge-1".to_owned(), "123456".to_owned())
            .expect("verify");
        assert_eq!(snapshot.phase, FieldSessionPhase::Authenticated);
        assert_eq!(
            snapshot
                .account
                .as_ref()
                .map(|account| account.username.as_str()),
            Some("field")
        );
        assert!(snapshot.access_token_present);
        assert!(snapshot.refresh_token_present);
        assert!(!format!("{snapshot:?}").contains("access-token"));
        let tokens = runtime
            .field_session_token_bundle()
            .expect("tokens")
            .expect("token bundle");
        assert_eq!(tokens.refresh_token, "refresh-token");
        handle.join().expect("server");
        runtime.stop();
    }

    #[test]
    fn restore_session_fetches_current_session_with_bearer_token() {
        let body = sample_session_json();
        let (base_url, handle) = spawn_response(
            "GET /v1/auth/session HTTP/1.1",
            Some("authorization: Bearer access-token"),
            body.as_str(),
        );
        let runtime = RadrootsRuntime::new().expect("runtime");
        runtime
            .field_configure_auth(Some(base_url), None)
            .expect("configure");

        let snapshot = runtime
            .field_restore_session("access-token".to_owned(), Some("refresh-token".to_owned()))
            .expect("restore");
        assert_eq!(snapshot.phase, FieldSessionPhase::Authenticated);
        assert_eq!(
            snapshot
                .session
                .as_ref()
                .map(|session| session.session_id.as_str()),
            Some("session-public-id")
        );
        handle.join().expect("server");
        runtime.stop();
    }

    fn spawn_response(
        expected_start: &'static str,
        expected_contains: Option<&'static str>,
        body: &str,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let body = body.to_owned();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = vec![0; 8192];
            let read = stream.read(&mut request).expect("read");
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with(expected_start), "{request}");
            if let Some(expected) = expected_contains {
                assert!(request.contains(expected), "{request}");
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).expect("write");
        });
        (format!("http://{addr}"), handle)
    }

    fn sample_session_bundle_json(access_token: &str, refresh_token: &str) -> String {
        format!(
            r#"{{"account":{},"profile":{},"credential":{},"session":{},"access_token":"{}","refresh_token":"{}"}}"#,
            sample_account_json(),
            sample_profile_json(),
            sample_credential_json(),
            sample_session_body_json(),
            access_token,
            refresh_token
        )
    }

    fn sample_session_json() -> String {
        format!(
            r#"{{"account":{},"profile":{},"credential":{},"session":{}}}"#,
            sample_account_json(),
            sample_profile_json(),
            sample_credential_json(),
            sample_session_body_json()
        )
    }

    fn sample_account_json() -> &'static str {
        r#"{"id":"account-1","username":"field","display_name":"Field Operator","status":"active"}"#
    }

    fn sample_profile_json() -> &'static str {
        r#"{"id":"profile-1","account_id":"account-1","display_name":"Field Operator","email":"field@radroots.test","status":"active"}"#
    }

    fn sample_credential_json() -> &'static str {
        r#"{"id":"credential-1","account_id":"account-1","profile_id":"profile-1","email":"field@radroots.test","status":"active","is_primary":true}"#
    }

    fn sample_session_body_json() -> &'static str {
        r#"{"id":"session-row-1","account_id":"account-1","profile_id":"profile-1","credential_id":"credential-1","session_id":"session-public-id","status":"active","expires_at_unix_seconds":1893456000,"revoked_at_unix_seconds":null}"#
    }
}
