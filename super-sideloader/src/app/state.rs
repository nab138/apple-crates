use crate::app::effects as app_effects;
use crate::app::effects::DeveloperSessionContext;
use crate::app::models::{
    AccountOption, AdiBackendKind, AdiBackendOption, AppOption, DeviceOption, MachineIdentity,
    SideloadOperation, TeamOption,
};
use crate::app::preferences::{
    load_preferences, save_preferences as save_preferences_to_disk, AdiPreferences,
    AppOverridePreferences, AppPreferences, DeveloperPreferences, MachineIdentityPreferences,
    SideloaderPreferences, StoredAdiBackendKind, ThemePreference,
};
use crate::app::selection::AppSelection;
use crate::device_selection::DeviceSelection;
use rand::RngExt;
use std::path::PathBuf;

pub(crate) struct LoadedSideloaderState {
    pub(crate) state: SideloaderState,
    pub(crate) app_path_to_restore: Option<PathBuf>,
    pub(crate) app_overrides_to_restore: AppOverridePreferences,
    pub(crate) should_save_preferences: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SideloadReadiness {
    Ready,
    NeedsAdi,
    NeedsAccount,
    NeedsTeam,
    NeedsCertificate,
    NeedsAppId,
    NeedsApp,
    NeedsDevice,
}

impl SideloadReadiness {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::NeedsAdi => "Set up CoreADI",
            Self::NeedsAccount => "Log in to your account",
            Self::NeedsTeam => "Select a developer team",
            Self::NeedsCertificate => "Set up a certificate",
            Self::NeedsAppId => "Select an App ID",
            Self::NeedsApp => "Select an IPA",
            Self::NeedsDevice => "Connect a device",
        }
    }
}

#[derive(Debug)]
pub(crate) struct SideloaderState {
    pub(crate) theme_preference: ThemePreference,
    pub(crate) accounts: Vec<AccountOption>,
    pub(crate) app_selection: AppSelection,
    pub(crate) device_selection: DeviceSelection,
    pub(crate) adi_backends: Vec<AdiBackendOption>,
    pub(crate) selected_account: usize,
    pub(crate) selected_team: usize,
    pub(crate) selected_certificate: usize,
    pub(crate) auto_app_id: bool,
    pub(crate) selected_app_id: usize,
    pub(crate) selected_adi_backend: usize,
    pub(crate) machine_identity: MachineIdentity,
    pub(crate) android_device_identity: MachineIdentity,
    pub(crate) android_device_identity_customized: bool,
    pub(crate) android_adi_identifier: String,
    pub(crate) enabled_patches: Vec<bool>,
    pub(crate) sideload_operation: SideloadOperation,
}

impl SideloaderState {
    pub(crate) fn load() -> LoadedSideloaderState {
        let preferences = load_preferences().unwrap_or_else(|error| {
            log::warn!("{error}");
            SideloaderPreferences::default()
        });
        let app_path_to_restore = preferences.app.path.as_ref().map(PathBuf::from);
        let app_overrides_to_restore = preferences.app.overrides.clone();
        let app_selection = AppSelection::default();
        let machine_identity = app_effects::load_machine_identity();
        let mut android_device_uuid = preferences
            .adi
            .android_device
            .machine_id
            .clone()
            .or_else(|| preferences.adi.android_device_uuid.clone())
            .unwrap_or_default();
        let generated_android_device_uuid = ensure_android_device_uuid(&mut android_device_uuid);
        let android_device_identity_customized = preferences.adi.android_device_identity_customized;
        let android_device_identity = android_adi_identity(
            android_device_uuid,
            &preferences.adi.android_device,
            android_device_identity_customized,
        );
        let mut android_adi_identifier = preferences
            .adi
            .android_adi_identifier
            .clone()
            .or_else(|| {
                preferences
                    .adi
                    .android_machine
                    .machine_id
                    .clone()
                    .filter(|identifier| identifier.len() == 16)
            })
            .unwrap_or_default();
        let generated_android_adi_identifier =
            ensure_android_adi_identifier(&mut android_adi_identifier);
        let device_selection = DeviceSelection::new(preferences.device.as_ref());
        let adi_backends = app_effects::available_adi_backends(&android_adi_identifier);
        let selected_adi_backend =
            selected_adi_backend_index(&adi_backends, preferences.adi.backend);
        let accounts = app_effects::load_cached_accounts().unwrap_or_else(|error| {
            log::warn!("{}", error.user_message());
            Vec::new()
        });
        let selected_account = selected_account_index(&accounts, &preferences.developer);
        let selected_team =
            selected_team_index(&accounts, selected_account, &preferences.developer);
        let selected_app_id = selected_app_id_index(
            &accounts,
            selected_account,
            selected_team,
            &preferences.developer,
        );
        let selected_certificate = selected_certificate_index(
            &accounts,
            selected_account,
            selected_team,
            &preferences.developer,
        );
        let enabled_patches = app_selection
            .selected()
            .map(|app| vec![false; app.patches.len()])
            .unwrap_or_default();

        LoadedSideloaderState {
            state: Self {
                theme_preference: preferences.theme,
                accounts,
                app_selection,
                device_selection,
                adi_backends,
                selected_account,
                selected_team,
                selected_certificate,
                auto_app_id: preferences.developer.auto_app_id,
                selected_app_id,
                selected_adi_backend,
                machine_identity,
                android_device_identity,
                android_device_identity_customized,
                android_adi_identifier,
                enabled_patches,
                sideload_operation: SideloadOperation::Idle,
            },
            app_path_to_restore,
            app_overrides_to_restore,
            should_save_preferences: generated_android_adi_identifier
                || generated_android_device_uuid,
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.sideload_operation.is_busy() || self.app_selection.is_loading()
    }

    pub(crate) fn save_preferences(&self) {
        if let Err(error) = save_preferences_to_disk(&self.preferences()) {
            log::warn!("{error}");
        }
    }

    pub(crate) fn selected_account(&self) -> Option<&AccountOption> {
        self.accounts.get(self.selected_account)
    }

    pub(crate) fn selected_team(&self) -> Option<&TeamOption> {
        self.selected_account()
            .and_then(|account| account.teams.get(self.selected_team))
    }

    pub(crate) fn selected_app(&self) -> Option<&AppOption> {
        self.app_selection.selected()
    }

    pub(crate) fn selected_device(&self) -> Option<&DeviceOption> {
        self.device_selection.selected()
    }

    pub(crate) fn sideload_readiness(&self) -> SideloadReadiness {
        if !self
            .adi_backends
            .get(self.selected_adi_backend)
            .is_some_and(|backend| backend.availability.is_ready())
        {
            return SideloadReadiness::NeedsAdi;
        }
        let Some(account) = self.selected_account() else {
            return SideloadReadiness::NeedsAccount;
        };
        let Some(team) = account.teams.get(self.selected_team) else {
            return SideloadReadiness::NeedsTeam;
        };
        let Some(certificate) = team.certificates.get(self.selected_certificate) else {
            return SideloadReadiness::NeedsCertificate;
        };
        if !certificate.private_key_available
            || certificate.certificate_fingerprint.is_none()
            || certificate.public_key_fingerprint.is_none()
        {
            return SideloadReadiness::NeedsCertificate;
        }
        if !self.auto_app_id && team.app_ids.get(self.selected_app_id).is_none() {
            return SideloadReadiness::NeedsAppId;
        }
        if self.selected_app().is_none() {
            return SideloadReadiness::NeedsApp;
        }
        if self.selected_device().is_none() {
            return SideloadReadiness::NeedsDevice;
        }
        SideloadReadiness::Ready
    }

    pub(crate) fn select_team(&mut self, index: usize) -> Option<String> {
        let team_count = self.accounts.get(self.selected_account)?.teams.len();
        if index >= team_count {
            return None;
        }

        self.selected_team = index;
        self.selected_certificate = 0;
        self.selected_app_id = 0;
        self.selected_team().map(|team| team.identifier.clone())
    }

    pub(crate) fn selected_developer_context(&self) -> Result<DeveloperSessionContext, String> {
        let account = self
            .accounts
            .get(self.selected_account)
            .ok_or_else(|| "No Apple Account is selected.".to_string())?;
        let adi_backend = self
            .adi_backends
            .get(self.selected_adi_backend)
            .filter(|backend| backend.availability.is_ready())
            .map(|backend| backend.kind)
            .ok_or_else(|| "No ready ADI backend is available.".to_string())?;
        let machine_identity = self.adi_machine_identity(adi_backend);

        Ok(DeveloperSessionContext::new(
            account.id.to_string(),
            account.apple_id.to_string(),
            adi_backend,
            machine_identity.clone(),
            self.android_adi_identifier.clone(),
        ))
    }

    pub(crate) fn adi_machine_identity(&self, backend: AdiBackendKind) -> &MachineIdentity {
        Self::machine_identity_for_adi_backend(
            backend,
            &self.machine_identity,
            &self.android_device_identity,
        )
    }

    pub(crate) fn machine_identity_for_adi_backend<'a>(
        backend: AdiBackendKind,
        host_identity: &'a MachineIdentity,
        android_identity: &'a MachineIdentity,
    ) -> &'a MachineIdentity {
        match backend {
            AdiBackendKind::AndroidCoreAdi => android_identity,
            AdiBackendKind::SystemAdid | AdiBackendKind::WindowsCoreAdi => host_identity,
        }
    }

    pub(crate) fn default_developer_app_id_fields(
        &self,
        team_id: &str,
    ) -> Result<(String, String), String> {
        let app = self
            .selected_app()
            .ok_or_else(|| "Select an IPA before creating an App ID.".to_string())?;
        let bundle_id = app.bundle_id().to_string();
        let identifier = if bundle_id
            .strip_prefix(team_id)
            .is_some_and(|suffix| suffix.starts_with('.'))
        {
            bundle_id
        } else {
            format!("{team_id}.{bundle_id}")
        };
        Ok((identifier, app.name().to_string()))
    }

    pub(crate) fn replace_developer_account_preserving_selection(
        &mut self,
        account: AccountOption,
    ) -> bool {
        let Some(selection) = replace_developer_account_preserving_selection(
            &mut self.accounts,
            account,
            DeveloperSelection {
                account: self.selected_account,
                team: self.selected_team,
                certificate: self.selected_certificate,
                app_id: self.selected_app_id,
            },
        ) else {
            return false;
        };
        self.selected_account = selection.account;
        self.selected_team = selection.team;
        self.selected_certificate = selection.certificate;
        self.selected_app_id = selection.app_id;
        true
    }

    pub(crate) fn add_developer_account(&mut self, account: AccountOption) {
        self.accounts.push(account);
        self.selected_account = self.accounts.len().saturating_sub(1);
        self.selected_team = 0;
        self.selected_certificate = 0;
        self.selected_app_id = 0;
    }

    pub(crate) fn log_out_selected_developer_account(&mut self) {
        let account_id = self
            .accounts
            .get(self.selected_account)
            .map(|account| account.id.to_string());
        if let Some(account_id) = account_id {
            if let Err(error) = app_effects::delete_account_cache(&account_id) {
                log::warn!("{}", error.user_message());
            }
            self.accounts
                .retain(|account| account.id.as_str() != account_id.as_str());
        }

        self.selected_account = self
            .selected_account
            .min(self.accounts.len().saturating_sub(1));
        self.selected_team = 0;
        self.selected_certificate = 0;
        self.selected_app_id = 0;
    }

    pub(crate) fn replace_app(&mut self, app_index: usize, app: AppOption) -> bool {
        self.app_selection.replace(app_index, app)
    }

    pub(crate) fn replace_adi_backends(
        &mut self,
        backends: Vec<AdiBackendOption>,
        selected_backend: usize,
    ) {
        self.adi_backends = backends;
        self.selected_adi_backend = selected_backend;
    }

    pub(crate) fn replace_android_device_identity(&mut self, identity: MachineIdentity) {
        self.android_device_identity = identity;
        self.android_device_identity_customized = true;
    }

    pub(crate) fn refresh_adi_backends(&mut self) {
        let selected_kind = self
            .adi_backends
            .get(self.selected_adi_backend)
            .map(|backend| backend.kind);
        self.adi_backends =
            app_effects::available_adi_backends_with_provisioning(&self.android_adi_identifier);
        self.selected_adi_backend = selected_kind
            .and_then(|kind| {
                self.adi_backends
                    .iter()
                    .position(|backend| backend.kind == kind)
            })
            .unwrap_or_else(|| app_effects::default_adi_backend(&self.adi_backends));
    }

    fn preferences(&self) -> SideloaderPreferences {
        let account = self.accounts.get(self.selected_account);
        let team = account.and_then(|account| account.teams.get(self.selected_team));
        let certificate = team.and_then(|team| team.certificates.get(self.selected_certificate));
        let app_id = team.and_then(|team| team.app_ids.get(self.selected_app_id));
        let app = self.selected_app();
        let backend = self.adi_backends.get(self.selected_adi_backend);

        SideloaderPreferences {
            theme: self.theme_preference,
            developer: DeveloperPreferences {
                account_id: account.map(|account| account.id.to_string()),
                team_id: team.map(|team| team.identifier.to_string()),
                certificate_serial_number: certificate
                    .map(|certificate| certificate.serial_number.to_string()),
                auto_app_id: self.auto_app_id,
                app_id: app_id.map(|app_id| app_id.identifier.to_string()),
            },
            app: AppPreferences {
                bundle_id: app.map(|app| app.bundle_id().to_string()),
                path: self.app_selection.selected_path_for_preferences(),
                overrides: app.map(AppOverridePreferences::from).unwrap_or_default(),
            },
            device: self.device_selection.selected_preferences(),
            adi: AdiPreferences {
                backend: backend.map(|backend| StoredAdiBackendKind::from(backend.kind)),
                machine: MachineIdentityPreferences::from(&self.machine_identity),
                android_adi_identifier: Some(self.android_adi_identifier.clone()),
                android_device: MachineIdentityPreferences::from(&self.android_device_identity),
                android_device_uuid: Some(self.android_device_identity.machine_id.to_string()),
                android_device_identity_customized: self.android_device_identity_customized,
                android_machine: MachineIdentityPreferences::default(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeveloperSelection {
    account: usize,
    team: usize,
    certificate: usize,
    app_id: usize,
}

fn replace_developer_account_preserving_selection(
    accounts: &mut [AccountOption],
    account: AccountOption,
    selection: DeveloperSelection,
) -> Option<DeveloperSelection> {
    let replacement_account_id = account.id.to_string();
    let previous_team_id = accounts
        .get(selection.account)?
        .teams
        .get(selection.team)
        .map(|team| team.identifier.to_string());
    let previous_certificate_serial_number = accounts
        .get(selection.account)?
        .teams
        .get(selection.team)
        .and_then(|team| team.certificates.get(selection.certificate))
        .map(|certificate| certificate.serial_number.to_string());
    let previous_app_id = accounts
        .get(selection.account)?
        .teams
        .get(selection.team)
        .and_then(|team| team.app_ids.get(selection.app_id))
        .map(|app_id| app_id.identifier.to_string());

    let account_index = accounts
        .iter()
        .position(|existing| existing.id.as_str() == replacement_account_id.as_str())?;
    accounts[account_index] = account;
    let team = previous_team_id
        .and_then(|team_id| {
            accounts[account_index]
                .teams
                .iter()
                .position(|team| team.identifier.as_str() == team_id)
        })
        .unwrap_or(0);
    let certificate = previous_certificate_serial_number
        .and_then(|serial_number| {
            accounts[account_index].teams.get(team).and_then(|team| {
                team.certificates
                    .iter()
                    .position(|candidate| candidate.serial_number.as_str() == serial_number)
            })
        })
        .unwrap_or(0);
    let app_id = previous_app_id
        .and_then(|app_id| {
            accounts[account_index].teams.get(team).and_then(|team| {
                team.app_ids
                    .iter()
                    .position(|candidate| candidate.identifier.as_str() == app_id)
            })
        })
        .unwrap_or(0);

    Some(DeveloperSelection {
        account: account_index,
        team,
        certificate,
        app_id,
    })
}

fn selected_account_index(accounts: &[AccountOption], preferences: &DeveloperPreferences) -> usize {
    preferences
        .account_id
        .as_deref()
        .and_then(|account_id| {
            accounts
                .iter()
                .position(|account| account.id.as_str() == account_id)
        })
        .unwrap_or(0)
}

fn selected_team_index(
    accounts: &[AccountOption],
    selected_account: usize,
    preferences: &DeveloperPreferences,
) -> usize {
    preferences
        .team_id
        .as_deref()
        .and_then(|team_id| {
            accounts
                .get(selected_account)?
                .teams
                .iter()
                .position(|team| team.identifier.as_str() == team_id)
        })
        .unwrap_or(0)
}

fn selected_app_id_index(
    accounts: &[AccountOption],
    selected_account: usize,
    selected_team: usize,
    preferences: &DeveloperPreferences,
) -> usize {
    preferences
        .app_id
        .as_deref()
        .and_then(|app_id| {
            accounts
                .get(selected_account)?
                .teams
                .get(selected_team)?
                .app_ids
                .iter()
                .position(|candidate| candidate.identifier.as_str() == app_id)
        })
        .unwrap_or(0)
}

fn selected_certificate_index(
    accounts: &[AccountOption],
    selected_account: usize,
    selected_team: usize,
    preferences: &DeveloperPreferences,
) -> usize {
    preferences
        .certificate_serial_number
        .as_deref()
        .and_then(|serial_number| {
            accounts
                .get(selected_account)?
                .teams
                .get(selected_team)?
                .certificates
                .iter()
                .position(|candidate| candidate.serial_number.as_str() == serial_number)
        })
        .unwrap_or(0)
}

fn selected_adi_backend_index(
    backends: &[AdiBackendOption],
    backend: Option<StoredAdiBackendKind>,
) -> usize {
    backend
        .and_then(|backend| {
            let backend = AdiBackendKind::from(backend);
            backends
                .iter()
                .position(|candidate| candidate.kind == backend)
        })
        .unwrap_or_else(|| app_effects::default_adi_backend(backends))
}

fn ensure_android_adi_identifier(identifier: &mut String) -> bool {
    if identifier.len() == 16 {
        return false;
    }

    *identifier = random_android_adi_identifier();
    true
}

fn ensure_android_device_uuid(identifier: &mut String) -> bool {
    if is_uuid(identifier) {
        return false;
    }

    *identifier = uuid::Uuid::new_v4().hyphenated().to_string().to_uppercase();
    true
}

fn is_uuid(identifier: &str) -> bool {
    uuid::Uuid::parse_str(identifier).is_ok()
}

fn default_android_adi_identity(device_uuid: String) -> MachineIdentity {
    MachineIdentity {
        machine_name: "MacBookPro13,2".to_string(),
        os_name: "macOS".to_string(),
        os_version: "13.1;22C65".to_string(),
        machine_id: device_uuid,
    }
}

fn android_adi_identity(
    device_uuid: String,
    preferences: &MachineIdentityPreferences,
    customized: bool,
) -> MachineIdentity {
    let mut identity = default_android_adi_identity(device_uuid);
    if customized {
        apply_machine_identity_preferences(&mut identity, preferences);
    }
    identity
}

fn apply_machine_identity_preferences(
    identity: &mut MachineIdentity,
    preferences: &MachineIdentityPreferences,
) {
    if let Some(machine_name) = preferences.machine_name.as_ref() {
        identity.machine_name = machine_name.clone();
    }
    if let Some(os_name) = preferences.os_name.as_ref() {
        identity.os_name = os_name.clone();
    }
    if let Some(os_version) = preferences.os_version.as_ref() {
        identity.os_version = os_version.clone();
    }
    if let Some(machine_id) = preferences.machine_id.as_ref() {
        identity.machine_id = machine_id.clone();
    }
}

fn random_android_adi_identifier() -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut rng = rand::rng();
    let mut id = String::with_capacity(16);
    for _ in 0..16 {
        let index: usize = rng.random_range(0..HEX.len());
        id.push(HEX[index] as char);
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::models::{
        AdiBackendAvailability, AdiBackendDetail, AdiProvisioningState, AppIdOption, AppMetadata,
        AppOption, DevelopmentCertificateOption, DeviceOption, EntitlementsSource,
        SupportedDeviceFamily,
    };
    use crate::app::AppError;
    use std::path::Path;

    fn app_id(identifier: &str) -> AppIdOption {
        AppIdOption {
            developer_id: format!("id-{identifier}"),
            name: identifier.to_string(),
            identifier: identifier.to_string(),
            kind: "Explicit".to_string(),
            capabilities: Vec::new(),
        }
    }

    fn certificate(serial_number: &str) -> DevelopmentCertificateOption {
        DevelopmentCertificateOption {
            id: format!("cert-{serial_number}"),
            name: serial_number.to_string(),
            serial_number: serial_number.to_string(),
            machine_name: "Mac".to_string(),
            private_key_available: true,
            certificate_fingerprint: Some(format!("certificate-fingerprint-{serial_number}")),
            public_key_fingerprint: Some(format!("fingerprint-{serial_number}")),
        }
    }

    fn team(
        identifier: &str,
        app_ids: Vec<AppIdOption>,
        certificates: Vec<DevelopmentCertificateOption>,
    ) -> TeamOption {
        TeamOption {
            name: identifier.to_string(),
            identifier: identifier.to_string(),
            role: "Admin".to_string(),
            app_id_available_quantity: Some(5),
            app_id_max_quantity: Some(10),
            app_ids,
            certificates,
        }
    }

    fn account(id: &str, teams: Vec<TeamOption>) -> AccountOption {
        AccountOption {
            id: id.to_string(),
            label: id.to_string(),
            apple_id: format!("{id}@example.com"),
            detail: "1 developer team".to_string(),
            status: "Ready".to_string(),
            teams,
        }
    }

    fn backend(kind: AdiBackendKind, availability: AdiBackendAvailability) -> AdiBackendOption {
        AdiBackendOption {
            kind,
            name: format!("{kind:?}"),
            detail: String::new(),
            availability,
            details: Vec::<AdiBackendDetail>::new(),
            provisioning_state: AdiProvisioningState::Unknown,
            editable_identity: false,
            repair_action: None,
        }
    }

    fn empty_state() -> SideloaderState {
        let identity = MachineIdentity {
            machine_name: "Mac".to_string(),
            os_name: "macOS".to_string(),
            os_version: "15".to_string(),
            machine_id: "machine-id".to_string(),
        };
        SideloaderState {
            theme_preference: ThemePreference::System,
            accounts: Vec::new(),
            app_selection: AppSelection::default(),
            device_selection: DeviceSelection::default(),
            adi_backends: Vec::new(),
            selected_account: 0,
            selected_team: 0,
            selected_certificate: 0,
            auto_app_id: true,
            selected_app_id: 0,
            selected_adi_backend: 0,
            machine_identity: identity.clone(),
            android_device_identity: identity,
            android_device_identity_customized: false,
            android_adi_identifier: "0123456789abcdef".to_string(),
            enabled_patches: Vec::new(),
            sideload_operation: SideloadOperation::Idle,
        }
    }

    fn sample_app() -> AppOption {
        AppOption {
            metadata: AppMetadata::sample(
                "Example",
                "com.example.app",
                "1.0",
                "1",
                "Example",
                "16.0",
                vec![SupportedDeviceFamily::IPhone],
            ),
            nested_bundles: Vec::new(),
            strip_extensions: false,
            path: "/tmp/Example.ipa".to_string(),
            icon_path: None,
            icon_override_path: None,
            entitlements: Vec::new(),
            entitlements_source: EntitlementsSource::GeneratedFallback,
            entitlement_overrides: None,
            patches: Vec::new(),
        }
    }

    #[test]
    fn sideload_readiness_reports_the_first_required_setup_step() {
        let mut state = empty_state();
        assert_eq!(state.sideload_readiness(), SideloadReadiness::NeedsAdi);

        state.adi_backends = vec![backend(
            AdiBackendKind::AndroidCoreAdi,
            AdiBackendAvailability::Ready,
        )];
        assert_eq!(state.sideload_readiness(), SideloadReadiness::NeedsAccount);

        state.accounts = vec![account("account", Vec::new())];
        assert_eq!(state.sideload_readiness(), SideloadReadiness::NeedsTeam);

        state.accounts[0].teams = vec![team("TEAM", Vec::new(), Vec::new())];
        assert_eq!(
            state.sideload_readiness(),
            SideloadReadiness::NeedsCertificate
        );

        state.accounts[0].teams[0].certificates = vec![certificate("SERIAL")];
        assert_eq!(state.sideload_readiness(), SideloadReadiness::NeedsApp);

        state.app_selection.finish_loading(
            Path::new("/tmp/Example.ipa"),
            Ok::<_, AppError>(sample_app()),
        );
        assert_eq!(state.sideload_readiness(), SideloadReadiness::NeedsDevice);

        let generation = state.device_selection.begin_refresh().unwrap();
        state.device_selection.finish_refresh(
            generation,
            Ok(vec![DeviceOption {
                name: "iPhone".to_string(),
                model: "iPhone17,1".to_string(),
                os: "iOS 19".to_string(),
                udid: "device-udid".to_string(),
                connection: "USB".to_string(),
            }]),
        );
        assert_eq!(state.sideload_readiness(), SideloadReadiness::Ready);

        state.accounts[0].teams[0].certificates[0].private_key_available = false;
        assert_eq!(
            state.sideload_readiness(),
            SideloadReadiness::NeedsCertificate
        );
    }

    #[test]
    fn restores_developer_selection_from_preferences() {
        let accounts = vec![
            account(
                "account-1",
                vec![team(
                    "TEAM1",
                    vec![app_id("TEAM1.com.example.one")],
                    vec![certificate("SERIAL1")],
                )],
            ),
            account(
                "account-2",
                vec![
                    team(
                        "TEAM2A",
                        vec![app_id("TEAM2A.com.example.app")],
                        vec![certificate("SERIAL2A")],
                    ),
                    team(
                        "TEAM2B",
                        vec![
                            app_id("TEAM2B.com.example.first"),
                            app_id("TEAM2B.com.example.selected"),
                        ],
                        vec![certificate("SERIAL2B-1"), certificate("SERIAL2B-2")],
                    ),
                ],
            ),
        ];
        let preferences = DeveloperPreferences {
            account_id: Some("account-2".to_string()),
            team_id: Some("TEAM2B".to_string()),
            certificate_serial_number: Some("SERIAL2B-2".to_string()),
            auto_app_id: false,
            app_id: Some("TEAM2B.com.example.selected".to_string()),
        };

        let selected_account = selected_account_index(&accounts, &preferences);
        let selected_team = selected_team_index(&accounts, selected_account, &preferences);
        let selected_certificate =
            selected_certificate_index(&accounts, selected_account, selected_team, &preferences);
        let selected_app_id =
            selected_app_id_index(&accounts, selected_account, selected_team, &preferences);

        assert_eq!(selected_account, 1);
        assert_eq!(selected_team, 1);
        assert_eq!(selected_certificate, 1);
        assert_eq!(selected_app_id, 1);
    }

    #[test]
    fn account_replacement_preserves_matching_selection_by_identity() {
        let mut accounts = vec![account(
            "account-1",
            vec![
                team(
                    "TEAM-A",
                    vec![app_id("TEAM-A.com.example.app")],
                    vec![certificate("SERIAL-A")],
                ),
                team(
                    "TEAM-B",
                    vec![
                        app_id("TEAM-B.com.example.first"),
                        app_id("TEAM-B.com.example.selected"),
                    ],
                    vec![certificate("SERIAL-B-1"), certificate("SERIAL-B-2")],
                ),
            ],
        )];
        let replacement = account(
            "account-1",
            vec![
                team(
                    "TEAM-B",
                    vec![
                        app_id("TEAM-B.com.example.selected"),
                        app_id("TEAM-B.com.example.new"),
                    ],
                    vec![certificate("SERIAL-B-2"), certificate("SERIAL-B-3")],
                ),
                team(
                    "TEAM-A",
                    vec![app_id("TEAM-A.com.example.app")],
                    vec![certificate("SERIAL-A")],
                ),
            ],
        );

        let selection = replace_developer_account_preserving_selection(
            &mut accounts,
            replacement,
            DeveloperSelection {
                account: 0,
                team: 1,
                certificate: 1,
                app_id: 1,
            },
        )
        .expect("matching account should be replaced");

        assert_eq!(
            selection,
            DeveloperSelection {
                account: 0,
                team: 0,
                certificate: 0,
                app_id: 0,
            }
        );
    }

    #[test]
    fn missing_developer_selection_falls_back_to_first_options() {
        let mut accounts = vec![account(
            "account-1",
            vec![team(
                "TEAM-A",
                vec![app_id("TEAM-A.com.example.app")],
                vec![certificate("SERIAL-A")],
            )],
        )];
        let replacement = account(
            "account-1",
            vec![team(
                "TEAM-C",
                vec![app_id("TEAM-C.com.example.app")],
                vec![certificate("SERIAL-C")],
            )],
        );

        let selection = replace_developer_account_preserving_selection(
            &mut accounts,
            replacement,
            DeveloperSelection {
                account: 0,
                team: 0,
                certificate: 0,
                app_id: 0,
            },
        )
        .expect("matching account should be replaced");

        assert_eq!(
            selection,
            DeveloperSelection {
                account: 0,
                team: 0,
                certificate: 0,
                app_id: 0,
            }
        );
    }

    #[test]
    fn android_adi_identity_is_only_generated_when_missing_or_invalid() {
        let mut adi_identifier = "0123456789abcdef".to_string();
        assert!(!ensure_android_adi_identifier(&mut adi_identifier));
        assert_eq!(adi_identifier, "0123456789abcdef");

        adi_identifier = "short".to_string();
        assert!(ensure_android_adi_identifier(&mut adi_identifier));
        assert_eq!(adi_identifier.len(), 16);

        let mut device_uuid = "550E8400-E29B-41D4-A716-446655440000".to_string();
        assert!(!ensure_android_device_uuid(&mut device_uuid));
        assert_eq!(device_uuid, "550E8400-E29B-41D4-A716-446655440000");

        device_uuid = "not-a-uuid".to_string();
        assert!(ensure_android_device_uuid(&mut device_uuid));
        assert!(uuid::Uuid::parse_str(&device_uuid).is_ok());
        assert_eq!(device_uuid, device_uuid.to_ascii_uppercase());
    }

    #[test]
    fn machine_identity_preferences_override_android_identity_fields() {
        let preferences = MachineIdentityPreferences {
            machine_name: Some("Android Device".to_string()),
            os_name: None,
            os_version: Some("14".to_string()),
            machine_id: Some("PERSISTED-ID".to_string()),
        };
        let default_identity = android_adi_identity("ANDROID-ID".to_string(), &preferences, false);

        assert_eq!(default_identity.machine_name, "MacBookPro13,2");
        assert_eq!(default_identity.os_name, "macOS");
        assert_eq!(default_identity.os_version, "13.1;22C65");
        assert_eq!(default_identity.machine_id, "ANDROID-ID");

        let android_identity = android_adi_identity("ANDROID-ID".to_string(), &preferences, true);

        assert_eq!(android_identity.machine_name, "Android Device");
        assert_eq!(android_identity.os_name, "macOS");
        assert_eq!(android_identity.os_version, "14");
        assert_eq!(android_identity.machine_id, "PERSISTED-ID");
    }

    #[test]
    fn restores_selected_adi_backend_from_preferences() {
        let backends = vec![
            backend(
                AdiBackendKind::SystemAdid,
                AdiBackendAvailability::Unavailable,
            ),
            backend(
                AdiBackendKind::AndroidCoreAdi,
                AdiBackendAvailability::Ready,
            ),
            backend(
                AdiBackendKind::WindowsCoreAdi,
                AdiBackendAvailability::NeedsSetup,
            ),
        ];

        assert_eq!(
            selected_adi_backend_index(&backends, Some(StoredAdiBackendKind::AndroidCoreAdi),),
            1
        );
        assert_eq!(selected_adi_backend_index(&backends, None), 1);
    }
}
