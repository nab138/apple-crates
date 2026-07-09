use crate::app::models::{
    AccountOption, AdiBackendKind, AdiBackendOption, AppOption, DeviceOption, MachineIdentity,
    PatchOption,
};
use crate::app::selection;
use crate::app::view_models::{
    account_option, account_options, adi_backends, app_option, device_options, domain_adi_kind,
    domain_machine_identity, domain_patch, machine_identity,
};
use crate::app::{AppError, AppResult};
use crate::backend::{adi_services, developer_services, device_discovery, system_identity};
use std::path::{Path, PathBuf};

pub(crate) use crate::domain::DeviceWatchEvent;
pub(crate) use adi_services::CoreAdiInstallEvent;
pub(crate) use developer_services::{
    DeveloperAppIdCapabilityUpdate, DownloadedProvisioningProfile,
};

#[derive(Clone, Debug)]
pub(crate) struct DeveloperSessionContext {
    inner: developer_services::DeveloperSessionContext,
}

impl DeveloperSessionContext {
    pub(crate) fn new(
        account_id: String,
        email: String,
        adi_backend: AdiBackendKind,
        machine_identity: MachineIdentity,
        android_adi_identifier: String,
    ) -> Self {
        Self {
            inner: developer_services::DeveloperSessionContext::new(
                account_id,
                email,
                domain_adi_kind(adi_backend),
                domain_machine_identity(&machine_identity),
                android_adi_identifier,
            ),
        }
    }

    fn into_backend(self) -> developer_services::DeveloperSessionContext {
        self.inner
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DeveloperLoginRequest {
    pub(crate) email: String,
    pub(crate) password: String,
    pub(crate) remember_account: bool,
    pub(crate) adi_backend: AdiBackendKind,
    pub(crate) machine_identity: MachineIdentity,
    pub(crate) android_adi_identifier: String,
}

impl DeveloperLoginRequest {
    fn into_backend(self) -> developer_services::DeveloperLoginRequest {
        developer_services::DeveloperLoginRequest {
            email: self.email,
            password: self.password,
            remember_account: self.remember_account,
            adi_backend: domain_adi_kind(self.adi_backend),
            machine_identity: domain_machine_identity(&self.machine_identity),
            android_adi_identifier: self.android_adi_identifier,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum DeveloperLoginOutcome {
    SignedIn(AccountOption),
    RequiresSecondaryAction { detail: String },
}

pub(crate) fn load_cached_accounts() -> AppResult<Vec<AccountOption>> {
    developer_services::load_cached_accounts()
        .map(account_options)
        .map_err(AppError::from)
}

pub(crate) fn delete_account_cache(account_id: &str) -> AppResult<()> {
    developer_services::delete_account_cache(account_id).map_err(AppError::from)
}

pub(crate) fn secondary_action_not_supported() -> String {
    developer_services::secondary_action_not_supported()
}

pub(crate) fn load_machine_identity() -> MachineIdentity {
    machine_identity(system_identity::machine_identity())
}

pub(crate) fn available_adi_backends(android_adi_identifier: &str) -> Vec<AdiBackendOption> {
    adi_backends(adi_services::available_backends(android_adi_identifier))
}

pub(crate) fn available_adi_backends_with_provisioning(
    android_adi_identifier: &str,
) -> Vec<AdiBackendOption> {
    adi_backends(adi_services::available_backends_with_provisioning(
        android_adi_identifier,
    ))
}

pub(crate) fn default_adi_backend(backends: &[AdiBackendOption]) -> usize {
    backends
        .iter()
        .position(|backend| backend.availability.is_ready())
        .unwrap_or(0)
}

pub(crate) async fn download_and_install_coreadi(
    progress: impl FnMut(CoreAdiInstallEvent) + Send + 'static,
) -> AppResult<PathBuf> {
    adi_services::download_and_install_coreadi(progress)
        .await
        .map_err(AppError::from)
}

pub(crate) async fn install_coreadi_from_apk(apk_path: PathBuf) -> AppResult<PathBuf> {
    adi_services::install_coreadi_from_apk(apk_path)
        .await
        .map_err(AppError::from)
}

pub(crate) fn erase_adi_provisioning(
    kind: AdiBackendKind,
    android_adi_identifier: &str,
) -> AppResult<()> {
    adi_services::erase_provisioning(domain_adi_kind(kind), android_adi_identifier)
        .map_err(AppError::from)
}

pub(crate) async fn provision_adi(
    kind: AdiBackendKind,
    machine_identity: &MachineIdentity,
    android_adi_identifier: &str,
) -> AppResult<()> {
    let machine_identity = domain_machine_identity(machine_identity);
    adi_services::provision(
        domain_adi_kind(kind),
        &machine_identity,
        android_adi_identifier,
    )
    .await
    .map_err(AppError::from)
}

pub(crate) async fn load_ipa(path: PathBuf, patches: Vec<PatchOption>) -> AppResult<AppOption> {
    let patches = patches.into_iter().map(domain_patch).collect();
    crate::backend::ipa::read_ipa(path, patches)
        .await
        .map(app_option)
        .map_err(AppError::from)
}

pub(crate) fn is_ipa_path(path: &Path) -> bool {
    selection::is_ipa_path(path)
}

pub(crate) async fn discover_devices() -> AppResult<Vec<DeviceOption>> {
    device_discovery::discover_devices()
        .await
        .map(device_options)
        .map_err(AppError::from)
}

pub(crate) async fn watch_device_changes(
    sender: futures::channel::mpsc::UnboundedSender<DeviceWatchEvent>,
) -> AppResult<()> {
    device_discovery::watch_device_changes(sender)
        .await
        .map_err(AppError::from)
}

pub(crate) fn open_app_data_folder() -> AppResult<()> {
    crate::backend::paths::open_app_data_folder().map_err(AppError::from)
}

pub(crate) fn save_provisioning_profile(
    folder: PathBuf,
    profile: DownloadedProvisioningProfile,
) -> AppResult<PathBuf> {
    crate::backend::paths::save_provisioning_profile(folder, &profile.name, profile.bytes)
        .map_err(AppError::from)
}

pub(crate) async fn login(request: DeveloperLoginRequest) -> AppResult<DeveloperLoginOutcome> {
    match developer_services::login(request.into_backend())
        .await
        .map_err(AppError::from)?
    {
        developer_services::DeveloperLoginOutcome::SignedIn(account) => {
            Ok(DeveloperLoginOutcome::SignedIn(account_option(account)))
        }
        developer_services::DeveloperLoginOutcome::RequiresSecondaryAction { detail } => {
            Ok(DeveloperLoginOutcome::RequiresSecondaryAction { detail })
        }
    }
}

pub(crate) async fn refresh_account(context: DeveloperSessionContext) -> AppResult<AccountOption> {
    developer_services::refresh_account(context.into_backend())
        .await
        .map(account_option)
        .map_err(AppError::from)
}

pub(crate) async fn add_app_id(
    context: DeveloperSessionContext,
    team_id: String,
    identifier: String,
    name: String,
) -> AppResult<AccountOption> {
    developer_services::add_app_id(context.into_backend(), team_id, identifier, name)
        .await
        .map(account_option)
        .map_err(AppError::from)
}

pub(crate) async fn update_app_id(
    context: DeveloperSessionContext,
    team_id: String,
    app_id_id: String,
    name: Option<String>,
    capabilities: Vec<DeveloperAppIdCapabilityUpdate>,
) -> AppResult<AccountOption> {
    developer_services::update_app_id(
        context.into_backend(),
        team_id,
        app_id_id,
        name,
        capabilities,
    )
    .await
    .map(account_option)
    .map_err(AppError::from)
}

pub(crate) async fn delete_app_id(
    context: DeveloperSessionContext,
    team_id: String,
    app_id_id: String,
) -> AppResult<AccountOption> {
    developer_services::delete_app_id(context.into_backend(), team_id, app_id_id)
        .await
        .map(account_option)
        .map_err(AppError::from)
}

pub(crate) async fn create_certificate(
    context: DeveloperSessionContext,
    team_id: String,
) -> AppResult<AccountOption> {
    developer_services::create_certificate(context.into_backend(), team_id)
        .await
        .map(account_option)
        .map_err(AppError::from)
}

pub(crate) async fn revoke_certificate(
    context: DeveloperSessionContext,
    team_id: String,
    serial_number: String,
) -> AppResult<AccountOption> {
    developer_services::revoke_certificate(context.into_backend(), team_id, serial_number)
        .await
        .map(account_option)
        .map_err(AppError::from)
}

pub(crate) async fn import_certificate_private_key(
    context: DeveloperSessionContext,
    team_id: String,
    certificate_id: String,
    public_key_fingerprint: String,
    private_key_path: PathBuf,
) -> AppResult<AccountOption> {
    developer_services::import_certificate_private_key(
        context.into_backend(),
        team_id,
        certificate_id,
        public_key_fingerprint,
        private_key_path,
    )
    .await
    .map(account_option)
    .map_err(AppError::from)
}

pub(crate) async fn download_provisioning_profile(
    context: DeveloperSessionContext,
    team_id: String,
    app_id_id: String,
) -> AppResult<DownloadedProvisioningProfile> {
    developer_services::download_provisioning_profile(context.into_backend(), team_id, app_id_id)
        .await
        .map_err(AppError::from)
}
