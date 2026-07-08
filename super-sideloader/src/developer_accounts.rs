use crate::models::{AccountOption, AppIdOption, TeamOption};
use crate::paths::app_data_dir;
use gpui::SharedString;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use uuid::Uuid;

const ACCOUNTS_DIR: &str = "accounts";
const ACCOUNT_INDEX_FILE: &str = "index.toml";
const CACHE_VERSION: u32 = 1;
const KEYRING_SERVICE: &str = "com.Dadoum.Super-Sideloader";

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
    pub(crate) last_refreshed_at: Option<String>,
    pub(crate) teams: Vec<CachedDeveloperTeam>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CachedDeveloperTeam {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) role: String,
    #[serde(default)]
    pub(crate) app_ids: Vec<CachedAppId>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CachedAppId {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: String,
}

impl Default for DeveloperAccountCache {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION,
            id: new_account_id(),
            email: String::new(),
            profile_name: None,
            token_expires_at: None,
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
            last_refreshed_at: account.last_refreshed_at.clone(),
        }
    }
}

impl From<DeveloperAccountCache> for AccountOption {
    fn from(account: DeveloperAccountCache) -> Self {
        let profile_name = account
            .profile_name
            .unwrap_or_else(|| account.email.clone());
        let status = account
            .token_expires_at
            .map(|expires_at| format!("Token expires {expires_at}"))
            .unwrap_or_else(|| "Cached".to_string());
        let detail = account
            .last_refreshed_at
            .map(|refreshed_at| format!("Last refreshed {refreshed_at}"))
            .unwrap_or_else(|| "Loaded from account cache".to_string());

        Self {
            id: account.id.into(),
            label: profile_name.into(),
            apple_id: account.email.into(),
            detail: detail.into(),
            status: status.into(),
            teams: account.teams.into_iter().map(TeamOption::from).collect(),
        }
    }
}

impl From<CachedDeveloperTeam> for TeamOption {
    fn from(team: CachedDeveloperTeam) -> Self {
        Self {
            name: team.name.into(),
            identifier: team.id.into(),
            role: team.role.into(),
            app_ids: team.app_ids.into_iter().map(AppIdOption::from).collect(),
        }
    }
}

impl From<CachedAppId> for AppIdOption {
    fn from(app_id: CachedAppId) -> Self {
        Self {
            name: app_id.name.into(),
            identifier: app_id.id.into(),
            kind: app_id.kind.into(),
        }
    }
}

pub(crate) fn load_account_options() -> Result<Vec<AccountOption>, String> {
    load_cached_accounts().map(|accounts| accounts.into_iter().map(AccountOption::from).collect())
}

pub(crate) fn load_cached_accounts() -> Result<Vec<DeveloperAccountCache>, String> {
    let index = load_account_index()?;
    let mut accounts = Vec::with_capacity(index.accounts.len());

    for entry in index.accounts {
        match load_account_cache_file(&entry.cache_file) {
            Ok(account) => accounts.push(account),
            Err(error) => eprintln!("{error}"),
        }
    }

    Ok(accounts)
}

#[allow(dead_code)]
pub(crate) fn save_account_cache(account: &DeveloperAccountCache) -> Result<String, String> {
    let mut account = account.clone();
    account.ensure_id();
    account.version = CACHE_VERSION;
    let account_id = account.id.clone();

    let cache_path = account_cache_path(&account.id)?;
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create account cache folder at {}: {error}",
                parent.display()
            )
        })?;
    }

    let contents = toml::to_string_pretty(&account)
        .map_err(|error| format!("Failed to encode account cache: {error}"))?;
    fs::write(&cache_path, contents).map_err(|error| {
        format!(
            "Failed to write account cache at {}: {error}",
            cache_path.display()
        )
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

#[allow(dead_code)]
pub(crate) fn delete_account_cache(account_id: &str) -> Result<(), String> {
    let cache_path = account_cache_path(account_id)?;
    match fs::remove_file(&cache_path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Failed to remove account cache at {}: {error}",
                cache_path.display()
            ));
        }
    }

    let mut index = load_account_index()?;
    index.accounts.retain(|account| account.id != account_id);
    save_account_index(&index)
}

pub(crate) fn new_account_id() -> String {
    Uuid::new_v4().to_string()
}

#[allow(dead_code)]
pub(crate) fn keyring_service() -> &'static str {
    KEYRING_SERVICE
}

#[allow(dead_code)]
pub(crate) fn keyring_account_name(account_id: &str) -> String {
    format!("account-session:{account_id}")
}

#[derive(Clone, Debug)]
pub(crate) enum MockDeveloperLoginOutcome {
    SignedIn(AccountOption),
    RequiresTwoFactor { detail: SharedString },
}

pub(crate) fn mock_developer_login(
    email: &str,
    password: &str,
) -> Result<MockDeveloperLoginOutcome, String> {
    if password.is_empty() {
        return Err("Enter the account password.".to_string());
    }

    match normalized_mock_email(email).as_deref() {
        Some("a@example.com") => Ok(MockDeveloperLoginOutcome::RequiresTwoFactor {
            detail: "Enter the six digit code sent to the trusted device.".into(),
        }),
        Some("b@example.com") => Ok(MockDeveloperLoginOutcome::SignedIn(mock_account(
            "b@example.com",
            "Mock Apple ID B",
            "Paid",
        ))),
        Some(_) | None => Err("No mock Apple Account matches that email.".to_string()),
    }
}

pub(crate) fn mock_developer_login_with_code(
    email: &str,
    code: &str,
) -> Result<AccountOption, String> {
    match normalized_mock_email(email).as_deref() {
        Some("a@example.com") if code.trim() == "123456" => {
            Ok(mock_account("a@example.com", "Mock Apple ID A", "Free"))
        }
        Some("a@example.com") => Err("The verification code is not valid.".to_string()),
        _ => Err("This mock account is not waiting for verification.".to_string()),
    }
}

fn normalized_mock_email(email: &str) -> Option<String> {
    let email = email.trim().to_ascii_lowercase();
    match email.as_str() {
        "a" => Some("a@example.com".to_string()),
        "b" => Some("b@example.com".to_string()),
        "" => None,
        _ => Some(email),
    }
}

fn mock_account(email: &str, label: &str, role: &str) -> AccountOption {
    let team_id = if role == "Paid" {
        "MOCKPAID01"
    } else {
        "MOCKFREE01"
    };

    AccountOption {
        id: new_account_id().into(),
        label: label.into(),
        apple_id: email.to_string().into(),
        detail: "Mock login session".into(),
        status: "Available".into(),
        teams: vec![TeamOption {
            name: "Mock Developer Team".into(),
            identifier: team_id.into(),
            role: role.into(),
            app_ids: vec![
                AppIdOption {
                    name: "Selected App".into(),
                    identifier: format!("{team_id}.com.example.app").into(),
                    kind: "Explicit App ID".into(),
                },
                AppIdOption {
                    name: "Wildcard Development".into(),
                    identifier: format!("{team_id}.*").into(),
                    kind: "Wildcard App ID".into(),
                },
            ],
        }],
    }
}

fn load_account_index() -> Result<DeveloperAccountIndex, String> {
    let path = account_index_path()?;
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(DeveloperAccountIndex {
                version: CACHE_VERSION,
                accounts: Vec::new(),
            });
        }
        Err(error) => {
            return Err(format!(
                "Failed to read account index at {}: {error}",
                path.display()
            ));
        }
    };

    let mut index: DeveloperAccountIndex = toml::from_str(&contents).map_err(|error| {
        format!(
            "Failed to parse account index at {}: {error}",
            path.display()
        )
    })?;
    index.version = CACHE_VERSION;
    Ok(index)
}

fn save_account_index(index: &DeveloperAccountIndex) -> Result<(), String> {
    let path = account_index_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create account index folder at {}: {error}",
                parent.display()
            )
        })?;
    }

    let mut index = index.clone();
    index.version = CACHE_VERSION;
    let contents = toml::to_string_pretty(&index)
        .map_err(|error| format!("Failed to encode account index: {error}"))?;
    fs::write(&path, contents).map_err(|error| {
        format!(
            "Failed to write account index at {}: {error}",
            path.display()
        )
    })
}

fn load_account_cache_file(cache_file: &str) -> Result<DeveloperAccountCache, String> {
    let path = accounts_dir()?.join(cache_file);
    let contents = fs::read_to_string(&path).map_err(|error| {
        format!(
            "Failed to read account cache at {}: {error}",
            path.display()
        )
    })?;
    let mut account: DeveloperAccountCache = toml::from_str(&contents).map_err(|error| {
        format!(
            "Failed to parse account cache at {}: {error}",
            path.display()
        )
    })?;
    account.version = CACHE_VERSION;
    Ok(account)
}

fn account_index_path() -> Result<PathBuf, String> {
    Ok(accounts_dir()?.join(ACCOUNT_INDEX_FILE))
}

fn account_cache_path(account_id: &str) -> Result<PathBuf, String> {
    Ok(accounts_dir()?.join(account_cache_file_name(account_id)))
}

fn accounts_dir() -> Result<PathBuf, String> {
    app_data_dir()
        .map(|path| path.join(ACCOUNTS_DIR))
        .ok_or_else(|| "The application data folder is not available.".to_string())
}

fn account_cache_file_name(account_id: &str) -> String {
    format!("{}.toml", account_id)
}
