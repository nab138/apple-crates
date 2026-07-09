use crate::backend::developer::keychain::{delete_keychain_session, token_is_near_expiry};
use crate::backend::paths::app_data_dir;
use crate::backend::{BackendError, BackendResult};
use crate::domain::{
    DeveloperAccount, DeveloperAppId, DeveloperAppIdCapability, DeveloperCertificate, DeveloperTeam,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use uuid::Uuid;

const ACCOUNTS_DIR: &str = "accounts";
const ACCOUNT_INDEX_FILE: &str = "index.toml";
pub(crate) const CACHE_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct DeveloperAccountIndex {
    pub(crate) version: u32,
    pub(crate) accounts: Vec<DeveloperAccountIndexEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DeveloperAccountIndexEntry {
    pub(crate) id: String,
    pub(crate) email: String,
    pub(crate) cache_file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) profile_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) token_expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) token_expires_at_epoch_millis: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) last_refreshed_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct DeveloperAccountCache {
    pub(crate) version: u32,
    pub(crate) id: String,
    pub(crate) email: String,
    pub(crate) profile_name: Option<String>,
    pub(crate) token_expires_at: Option<String>,
    pub(crate) token_expires_at_epoch_millis: Option<u64>,
    pub(crate) last_refreshed_at: Option<String>,
    pub(crate) teams: Vec<CachedDeveloperTeam>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct CachedDeveloperTeam {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) app_id_available_quantity: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) app_id_max_quantity: Option<u64>,
    #[serde(default)]
    pub(crate) app_ids: Vec<CachedAppId>,
    #[serde(default)]
    pub(crate) certificates: Vec<CachedDevelopmentCertificate>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct CachedAppId {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) developer_id: String,
    pub(crate) name: String,
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) capabilities: Vec<CachedAppIdCapability>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct CachedAppIdCapability {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) detail: String,
    pub(crate) enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct CachedDevelopmentCertificate {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) serial_number: String,
    pub(crate) machine_name: String,
    pub(crate) private_key_available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) public_key_fingerprint: Option<String>,
}

impl Default for DeveloperAccountCache {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION,
            id: new_account_id(),
            email: String::new(),
            profile_name: None,
            token_expires_at: None,
            token_expires_at_epoch_millis: None,
            last_refreshed_at: None,
            teams: Vec::new(),
        }
    }
}

impl DeveloperAccountCache {
    pub(crate) fn ensure_id(&mut self) -> &str {
        if self.id.is_empty() {
            self.id = new_account_id();
        }
        &self.id
    }
}

impl From<&DeveloperAccountCache> for DeveloperAccountIndexEntry {
    fn from(account: &DeveloperAccountCache) -> Self {
        Self {
            id: account.id.clone(),
            email: account.email.clone(),
            cache_file: account_cache_file_name(&account.id),
            profile_name: account.profile_name.clone(),
            token_expires_at: account.token_expires_at.clone(),
            token_expires_at_epoch_millis: account.token_expires_at_epoch_millis,
            last_refreshed_at: account.last_refreshed_at.clone(),
        }
    }
}

impl From<DeveloperAccountCache> for DeveloperAccount {
    fn from(account: DeveloperAccountCache) -> Self {
        Self {
            id: account.id,
            email: account.email,
            profile_name: account.profile_name,
            token_expires_at: account.token_expires_at,
            token_expires_at_epoch_millis: account.token_expires_at_epoch_millis,
            last_refreshed_at: account.last_refreshed_at,
            teams: account.teams.into_iter().map(DeveloperTeam::from).collect(),
        }
    }
}

impl From<CachedDeveloperTeam> for DeveloperTeam {
    fn from(team: CachedDeveloperTeam) -> Self {
        Self {
            name: team.name,
            id: team.id,
            role: team.role,
            app_id_available_quantity: team.app_id_available_quantity,
            app_id_max_quantity: team.app_id_max_quantity,
            app_ids: team.app_ids.into_iter().map(DeveloperAppId::from).collect(),
            certificates: team
                .certificates
                .into_iter()
                .map(DeveloperCertificate::from)
                .collect(),
        }
    }
}

impl From<CachedAppId> for DeveloperAppId {
    fn from(app_id: CachedAppId) -> Self {
        Self {
            developer_id: app_id.developer_id,
            name: app_id.name,
            id: app_id.id,
            kind: app_id.kind,
            capabilities: app_id
                .capabilities
                .into_iter()
                .map(DeveloperAppIdCapability::from)
                .collect(),
        }
    }
}

impl From<CachedAppIdCapability> for DeveloperAppIdCapability {
    fn from(capability: CachedAppIdCapability) -> Self {
        Self {
            key: capability.key,
            label: capability.label,
            detail: capability.detail,
            enabled: capability.enabled,
        }
    }
}

impl From<CachedDevelopmentCertificate> for DeveloperCertificate {
    fn from(certificate: CachedDevelopmentCertificate) -> Self {
        Self {
            id: certificate.id,
            name: certificate.name,
            serial_number: certificate.serial_number,
            machine_name: certificate.machine_name,
            private_key_available: certificate.private_key_available,
            public_key_fingerprint: certificate.public_key_fingerprint,
        }
    }
}

pub(crate) fn load_cached_account_options() -> BackendResult<Vec<DeveloperAccount>> {
    Ok(load_cached_account_files()?
        .into_iter()
        .filter_map(|account| {
            if account
                .token_expires_at_epoch_millis
                .is_some_and(token_is_near_expiry)
            {
                if let Err(error) = delete_account_cache(&account.id) {
                    log::warn!("{error}");
                }
                None
            } else {
                Some(account.into())
            }
        })
        .collect())
}

pub(crate) fn load_cached_account_files() -> BackendResult<Vec<DeveloperAccountCache>> {
    let index = load_account_index()?;
    let mut accounts = Vec::with_capacity(index.accounts.len());

    for entry in index.accounts {
        match load_account_cache_file(&entry.cache_file) {
            Ok(account) => accounts.push(account),
            Err(error) => log::warn!("{error}"),
        }
    }

    Ok(accounts)
}

pub(crate) fn load_account_cache_by_id(account_id: &str) -> BackendResult<DeveloperAccountCache> {
    load_account_cache_file(&account_cache_file_name(account_id))
}

pub(crate) fn refresh_account_seed(account_id: &str, email: &str) -> DeveloperAccountCache {
    load_account_cache_by_id(account_id).unwrap_or_else(|_| DeveloperAccountCache {
        id: account_id.to_string(),
        email: email.to_string(),
        ..DeveloperAccountCache::default()
    })
}

pub(crate) fn save_account_cache(account: &DeveloperAccountCache) -> BackendResult<String> {
    let mut account = account.clone();
    account.ensure_id();
    account.version = CACHE_VERSION;
    let account_id = account.id.clone();

    let cache_path = account_cache_path(&account.id)?;
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|source| BackendError::Io {
            action: "Create account cache folder",
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let contents = toml::to_string_pretty(&account)
        .map_err(|error| BackendError::Cache(format!("Failed to encode account cache: {error}")))?;
    fs::write(&cache_path, contents).map_err(|source| BackendError::Io {
        action: "Write account cache",
        path: cache_path.clone(),
        source,
    })?;

    let mut index = load_account_index()?;
    let entry = DeveloperAccountIndexEntry::from(&account);
    if let Some(existing) = index
        .accounts
        .iter_mut()
        .find(|existing| existing.id == account.id)
    {
        *existing = entry;
    } else {
        index.accounts.push(entry);
    }
    save_account_index(&index)?;
    Ok(account_id)
}

pub(crate) fn delete_account_cache(account_id: &str) -> BackendResult<()> {
    if let Err(error) = delete_keychain_session(account_id) {
        log::warn!("{error}");
    }

    let cache_path = account_cache_path(account_id)?;
    match fs::remove_file(&cache_path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(source) => {
            return Err(BackendError::Io {
                action: "Remove account cache",
                path: cache_path,
                source,
            });
        }
    }

    let mut index = load_account_index()?;
    index.accounts.retain(|account| account.id != account_id);
    save_account_index(&index)
}

pub(crate) fn merge_team_resources(
    account: &mut DeveloperAccountCache,
    refreshed_team: CachedDeveloperTeam,
) {
    if let Some(existing) = account
        .teams
        .iter_mut()
        .find(|team| team.id == refreshed_team.id)
    {
        *existing = refreshed_team;
    } else {
        account.teams.push(refreshed_team);
    }
}

pub(crate) fn new_account_id() -> String {
    Uuid::new_v4().to_string()
}

fn load_account_index() -> BackendResult<DeveloperAccountIndex> {
    let path = account_index_path()?;
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(DeveloperAccountIndex {
                version: CACHE_VERSION,
                accounts: Vec::new(),
            });
        }
        Err(source) => {
            return Err(BackendError::Io {
                action: "Read account index",
                path,
                source,
            });
        }
    };

    let mut index: DeveloperAccountIndex = toml::from_str(&contents).map_err(|error| {
        BackendError::Cache(format!(
            "Failed to parse account index at {}: {error}",
            path.display()
        ))
    })?;
    index.version = CACHE_VERSION;
    Ok(index)
}

fn save_account_index(index: &DeveloperAccountIndex) -> BackendResult<()> {
    let path = account_index_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| BackendError::Io {
            action: "Create account index folder",
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let mut index = index.clone();
    index.version = CACHE_VERSION;
    let contents = toml::to_string_pretty(&index)
        .map_err(|error| BackendError::Cache(format!("Failed to encode account index: {error}")))?;
    fs::write(&path, contents).map_err(|source| BackendError::Io {
        action: "Write account index",
        path,
        source,
    })
}

fn load_account_cache_file(cache_file: &str) -> BackendResult<DeveloperAccountCache> {
    let path = accounts_dir()?.join(cache_file);
    let contents = fs::read_to_string(&path).map_err(|source| BackendError::Io {
        action: "Read account cache",
        path: path.clone(),
        source,
    })?;
    let mut account: DeveloperAccountCache = toml::from_str(&contents).map_err(|error| {
        BackendError::Cache(format!(
            "Failed to parse account cache at {}: {error}",
            path.display()
        ))
    })?;
    account.version = CACHE_VERSION;
    Ok(account)
}

fn account_index_path() -> BackendResult<PathBuf> {
    Ok(accounts_dir()?.join(ACCOUNT_INDEX_FILE))
}

fn account_cache_path(account_id: &str) -> BackendResult<PathBuf> {
    Ok(accounts_dir()?.join(account_cache_file_name(account_id)))
}

fn accounts_dir() -> BackendResult<PathBuf> {
    app_data_dir()
        .map(|path| path.join(ACCOUNTS_DIR))
        .ok_or_else(|| {
            BackendError::Cache("The application data folder is not available.".to_string())
        })
}

fn account_cache_file_name(account_id: &str) -> String {
    format!("{account_id}.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn team(id: &str, name: &str) -> CachedDeveloperTeam {
        CachedDeveloperTeam {
            id: id.to_string(),
            name: name.to_string(),
            role: "Individual".to_string(),
            app_id_available_quantity: Some(1),
            app_id_max_quantity: Some(10),
            app_ids: vec![CachedAppId {
                id: format!("{id}.bundle"),
                developer_id: format!("{id}-app"),
                name: name.to_string(),
                kind: "Explicit App ID".to_string(),
                capabilities: Vec::new(),
            }],
            certificates: vec![CachedDevelopmentCertificate {
                id: format!("{id}-cert"),
                name: name.to_string(),
                serial_number: id.to_string(),
                machine_name: "Machine".to_string(),
                private_key_available: false,
                public_key_fingerprint: None,
            }],
        }
    }

    #[test]
    fn merge_team_resources_replaces_only_matching_team() {
        let mut account = DeveloperAccountCache {
            teams: vec![team("TEAM1", "Old"), team("TEAM2", "Untouched")],
            ..DeveloperAccountCache::default()
        };

        merge_team_resources(&mut account, team("TEAM1", "New"));

        assert_eq!(account.teams.len(), 2);
        assert_eq!(account.teams[0].name, "New");
        assert_eq!(account.teams[1].name, "Untouched");
    }

    #[test]
    fn merge_team_resources_adds_missing_team() {
        let mut account = DeveloperAccountCache {
            teams: vec![team("TEAM1", "Existing")],
            ..DeveloperAccountCache::default()
        };

        merge_team_resources(&mut account, team("TEAM2", "Added"));

        assert_eq!(account.teams.len(), 2);
        assert_eq!(account.teams[1].id, "TEAM2");
    }
}
