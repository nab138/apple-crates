use crate::backend::developer::accounts;
use crate::backend::developer::certificates::{self, AppManagedSigningMaterial};
use crate::backend::BackendResult;
use crate::domain::{AdiBackendKind, DeveloperAccount, DeveloperDevice, MachineIdentity};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct DeveloperSessionContext {
    account_id: String,
    email: String,
    adi_backend: AdiBackendKind,
    machine_identity: MachineIdentity,
    android_adi_identifier: String,
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
            account_id,
            email,
            adi_backend,
            machine_identity,
            android_adi_identifier,
        }
    }

    fn refresh_request(&self) -> accounts::DeveloperAccountRefreshRequest {
        accounts::DeveloperAccountRefreshRequest {
            account_id: self.account_id.clone(),
            email: self.email.clone(),
            adi_backend: self.adi_backend,
            machine_identity: self.machine_identity.clone(),
            android_adi_identifier: self.android_adi_identifier.clone(),
        }
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

#[derive(Clone, Debug)]
pub(crate) struct DeveloperAppIdCapabilityUpdate {
    pub(crate) key: String,
    pub(crate) enabled: bool,
}

pub(crate) use accounts::{DeveloperLoginOutcome, DownloadedProvisioningProfile};

pub(crate) fn load_cached_accounts() -> BackendResult<Vec<DeveloperAccount>> {
    accounts::load_cached_account_options()
}

pub(crate) fn delete_account_cache(account_id: &str) -> BackendResult<()> {
    accounts::delete_account_cache(account_id)
}

pub(crate) fn load_signing_material(
    certificate_fingerprint: &str,
    public_key_fingerprint: &str,
) -> BackendResult<AppManagedSigningMaterial> {
    certificates::load_app_managed_signing_material(certificate_fingerprint, public_key_fingerprint)
}

pub(crate) async fn login(request: DeveloperLoginRequest) -> BackendResult<DeveloperLoginOutcome> {
    accounts::login_developer_account(accounts::DeveloperLoginRequest {
        email: request.email,
        password: request.password,
        remember_account: request.remember_account,
        adi_backend: request.adi_backend,
        machine_identity: request.machine_identity,
        android_adi_identifier: request.android_adi_identifier,
    })
    .await
}

pub(crate) fn secondary_action_not_supported() -> String {
    accounts::developer_secondary_action_not_supported()
}

pub(crate) async fn refresh_account(
    context: DeveloperSessionContext,
) -> BackendResult<DeveloperAccount> {
    accounts::refresh_developer_account(context.refresh_request()).await
}

pub(crate) async fn add_app_id(
    context: DeveloperSessionContext,
    team_id: String,
    identifier: String,
    name: String,
) -> BackendResult<DeveloperAccount> {
    let request = context.refresh_request();
    accounts::add_developer_app_id(accounts::DeveloperAppIdAddRequest {
        account_id: request.account_id,
        email: request.email,
        team_id,
        identifier,
        name,
        adi_backend: request.adi_backend,
        machine_identity: request.machine_identity,
        android_adi_identifier: request.android_adi_identifier,
    })
    .await
}

pub(crate) async fn update_app_id(
    context: DeveloperSessionContext,
    team_id: String,
    app_id_id: String,
    name: Option<String>,
    capabilities: Vec<DeveloperAppIdCapabilityUpdate>,
) -> BackendResult<DeveloperAccount> {
    let request = context.refresh_request();
    accounts::update_developer_app_id(accounts::DeveloperAppIdUpdateRequest {
        account_id: request.account_id,
        email: request.email,
        team_id,
        app_id_id,
        name,
        capabilities: capabilities
            .into_iter()
            .map(|capability| accounts::DeveloperAppIdCapabilityUpdate {
                key: capability.key,
                enabled: capability.enabled,
            })
            .collect(),
        adi_backend: request.adi_backend,
        machine_identity: request.machine_identity,
        android_adi_identifier: request.android_adi_identifier,
    })
    .await
}

pub(crate) async fn delete_app_id(
    context: DeveloperSessionContext,
    team_id: String,
    app_id_id: String,
) -> BackendResult<DeveloperAccount> {
    let request = context.refresh_request();
    accounts::delete_developer_app_id(accounts::DeveloperAppIdDeleteRequest {
        account_id: request.account_id,
        email: request.email,
        team_id,
        app_id_id,
        adi_backend: request.adi_backend,
        machine_identity: request.machine_identity,
        android_adi_identifier: request.android_adi_identifier,
    })
    .await
}

pub(crate) async fn list_developer_devices(
    context: DeveloperSessionContext,
    team_id: String,
) -> BackendResult<Vec<DeveloperDevice>> {
    let request = context.refresh_request();
    accounts::list_developer_devices(accounts::DeveloperDeviceListRequest {
        account_id: request.account_id,
        email: request.email,
        team_id,
        adi_backend: request.adi_backend,
        machine_identity: request.machine_identity,
        android_adi_identifier: request.android_adi_identifier,
    })
    .await
}

pub(crate) async fn add_developer_device(
    context: DeveloperSessionContext,
    team_id: String,
    name: String,
    udid: String,
) -> BackendResult<Vec<DeveloperDevice>> {
    let request = context.refresh_request();
    accounts::add_developer_device(accounts::DeveloperDeviceAddRequest {
        account_id: request.account_id,
        email: request.email,
        team_id,
        name,
        udid,
        adi_backend: request.adi_backend,
        machine_identity: request.machine_identity,
        android_adi_identifier: request.android_adi_identifier,
    })
    .await
}

pub(crate) async fn delete_developer_device(
    context: DeveloperSessionContext,
    team_id: String,
    device_id: String,
) -> BackendResult<Vec<DeveloperDevice>> {
    let request = context.refresh_request();
    accounts::delete_developer_device(accounts::DeveloperDeviceDeleteRequest {
        account_id: request.account_id,
        email: request.email,
        team_id,
        device_id,
        adi_backend: request.adi_backend,
        machine_identity: request.machine_identity,
        android_adi_identifier: request.android_adi_identifier,
    })
    .await
}

pub(crate) async fn create_certificate(
    context: DeveloperSessionContext,
    team_id: String,
) -> BackendResult<DeveloperAccount> {
    let request = context.refresh_request();
    accounts::create_developer_certificate(accounts::DeveloperCertificateCreateRequest {
        account_id: request.account_id,
        email: request.email,
        team_id,
        adi_backend: request.adi_backend,
        machine_identity: request.machine_identity,
        android_adi_identifier: request.android_adi_identifier,
    })
    .await
}

pub(crate) async fn revoke_certificate(
    context: DeveloperSessionContext,
    team_id: String,
    serial_number: String,
) -> BackendResult<DeveloperAccount> {
    let request = context.refresh_request();
    accounts::revoke_developer_certificate(accounts::DeveloperCertificateRevokeRequest {
        account_id: request.account_id,
        email: request.email,
        team_id,
        serial_number,
        adi_backend: request.adi_backend,
        machine_identity: request.machine_identity,
        android_adi_identifier: request.android_adi_identifier,
    })
    .await
}

pub(crate) async fn import_certificate_private_key(
    context: DeveloperSessionContext,
    team_id: String,
    certificate_id: String,
    public_key_fingerprint: String,
    private_key_path: PathBuf,
) -> BackendResult<DeveloperAccount> {
    let request = context.refresh_request();
    accounts::import_developer_certificate_private_key(
        accounts::DeveloperCertificatePrivateKeyImportRequest {
            account_id: request.account_id,
            email: request.email,
            team_id,
            certificate_id,
            public_key_fingerprint,
            adi_backend: request.adi_backend,
            machine_identity: request.machine_identity,
            android_adi_identifier: request.android_adi_identifier,
        },
        private_key_path,
    )
    .await
}

pub(crate) async fn download_provisioning_profile(
    context: DeveloperSessionContext,
    team_id: String,
    app_id_id: String,
) -> BackendResult<DownloadedProvisioningProfile> {
    let request = context.refresh_request();
    accounts::download_developer_provisioning_profile(
        accounts::DeveloperProvisioningProfileDownloadRequest {
            account_id: request.account_id,
            email: request.email,
            team_id,
            app_id_id,
            adi_backend: request.adi_backend,
            machine_identity: request.machine_identity,
            android_adi_identifier: request.android_adi_identifier,
        },
    )
    .await
}
