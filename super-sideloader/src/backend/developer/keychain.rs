use crate::backend::{BackendError, BackendResult};
use grandslam::{AuthToken, Token};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const KEYCHAIN_SESSION_VERSION: u32 = 1;
const KEYRING_SERVICE: &str = "com.Dadoum.Super-Sideloader";
const TOKEN_EXPIRY_GRACE_MILLIS: u64 = 5 * 60 * 1000;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DeveloperAccountKeychainSession {
    pub(crate) version: u32,
    pub(crate) auth_token: AuthToken,
    pub(crate) heartbeat_token: Token,
    pub(crate) xcode_token: Token,
}

impl DeveloperAccountKeychainSession {
    pub(crate) fn new(auth_token: AuthToken, heartbeat_token: Token, xcode_token: Token) -> Self {
        Self {
            version: KEYCHAIN_SESSION_VERSION,
            auth_token,
            heartbeat_token,
            xcode_token,
        }
    }

    pub(crate) fn expires_at_millis(&self) -> u64 {
        self.heartbeat_token.expiry_epoch_millis
    }
}

pub(crate) fn save_keychain_session(
    account_id: &str,
    session: &DeveloperAccountKeychainSession,
) -> BackendResult<()> {
    let contents = toml::to_string(session).map_err(|error| {
        BackendError::Keychain(format!(
            "Failed to encode account keychain payload: {error}"
        ))
    })?;
    keyring_entry(account_id)?
        .set_password(&contents)
        .map_err(|error| {
            BackendError::Keychain(format!(
                "Failed to save account session to the system keychain: {error}"
            ))
        })
}

pub(crate) fn load_keychain_session(
    account_id: &str,
) -> BackendResult<Option<DeveloperAccountKeychainSession>> {
    let contents = match keyring_entry(account_id)?.get_password() {
        Ok(contents) => contents,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(error) => {
            return Err(BackendError::Keychain(format!(
                "Failed to read account session from the system keychain: {error}"
            )));
        }
    };
    let mut session: DeveloperAccountKeychainSession =
        toml::from_str(&contents).map_err(|error| {
            BackendError::Keychain(format!(
                "Failed to decode account keychain payload: {error}"
            ))
        })?;
    session.version = KEYCHAIN_SESSION_VERSION;
    Ok(Some(session))
}

pub(crate) fn delete_keychain_session(account_id: &str) -> BackendResult<()> {
    match keyring_entry(account_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(BackendError::Keychain(format!(
            "Failed to remove account session from the system keychain: {error}"
        ))),
    }
}

#[allow(dead_code)]
pub(crate) fn keyring_service() -> &'static str {
    KEYRING_SERVICE
}

#[allow(dead_code)]
pub(crate) fn keyring_account_name(account_id: &str) -> String {
    format!("account-session:{account_id}")
}

pub(crate) fn token_is_near_expiry(expiry_epoch_millis: u64) -> bool {
    if expiry_epoch_millis == 0 {
        return false;
    }
    current_epoch_millis().saturating_add(TOKEN_EXPIRY_GRACE_MILLIS) >= expiry_epoch_millis
}

pub(crate) fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn keyring_entry(account_id: &str) -> BackendResult<keyring::Entry> {
    keyring::Entry::new(keyring_service(), &keyring_account_name(account_id)).map_err(|error| {
        BackendError::Keychain(format!("Failed to access the system keychain: {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_expiry_is_treated_as_unknown_not_expired() {
        assert!(!token_is_near_expiry(0));
    }

    #[test]
    fn near_expiry_is_detected() {
        assert!(token_is_near_expiry(current_epoch_millis() + 1));
    }
}
