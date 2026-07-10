use crate::app::entitlements::{effective_bundle_identifier, effective_nested_bundle_identifier};
use crate::app::models::{
    AccountOption, AdiBackendKind, AdiBackendOption, AppIdOption, AppOption, DeveloperDeviceOption,
    DeviceOption, MachineIdentity, PatchOption, TeamOption,
};
use crate::app::selection;
use crate::app::view_models::{
    account_option, account_options, adi_backends, app_option, developer_device_options,
    device_options, domain_adi_kind, domain_app_metadata, domain_machine_identity, domain_patch,
    machine_identity,
};
use crate::app::{AppError, AppResult};
use crate::backend::{adi_services, developer_services, device_discovery, system_identity};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(crate) use crate::domain::DeviceWatchEvent;
pub(crate) use adi_services::CoreAdiInstallEvent;
pub(crate) use developer_services::{
    DeveloperAppIdCapabilityUpdate, DownloadedProvisioningProfile,
};

#[derive(Clone)]
pub(crate) struct SignIpaRequest {
    pub(crate) developer_context: DeveloperSessionContext,
    pub(crate) team_id: String,
    pub(crate) team_app_ids: Vec<AppIdOption>,
    pub(crate) certificate_fingerprint: String,
    pub(crate) public_key_fingerprint: String,
    pub(crate) auto_app_id: bool,
    pub(crate) selected_app_id: Option<AppIdOption>,
    pub(crate) app: AppOption,
    pub(crate) output: SignIpaOutput,
}

pub(crate) struct SignIpaOutcome {
    pub(crate) artifact: SignIpaArtifact,
    pub(crate) updated_account: Option<AccountOption>,
}

#[derive(Clone)]
pub(crate) enum SignIpaOutput {
    Ipa(PathBuf),
    AppBundle,
}

pub(crate) enum SignIpaArtifact {
    Ipa(PathBuf),
    AppBundle(crate::backend::ipa::SignedAppBundle),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AppIdProvisioningItem {
    pub(crate) name: String,
    pub(crate) identifier: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AppIdProvisioningPlan {
    pub(crate) app_ids: Vec<AppIdProvisioningItem>,
    pub(crate) available_quantity: Option<u64>,
}

impl AppIdProvisioningPlan {
    pub(crate) fn remaining_after_signing(&self) -> Option<u64> {
        self.available_quantity
            .map(|available| available.saturating_sub(self.app_ids.len() as u64))
    }
}

#[derive(Clone, Debug)]
struct SigningAppIdTarget {
    registration_identifier: String,
    bundle_identifier: String,
    name: String,
    existing: Option<AppIdOption>,
}

#[derive(Clone, Debug)]
struct SigningAppIdTargets {
    root: SigningAppIdTarget,
    nested: Vec<SigningAppIdTarget>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SignIpaProgress {
    Preparing,
    ResolvingAppId,
    DownloadingProfile,
    Extracting { completed: usize, total: usize },
    Patching,
    Signing,
    Packaging { completed: usize, total: usize },
    Saving,
    Ready,
}

impl SignIpaProgress {
    pub(crate) fn progress(self) -> f32 {
        match self {
            Self::Preparing => 0.04,
            Self::ResolvingAppId => 0.1,
            Self::DownloadingProfile => 0.22,
            Self::Extracting { completed, total } => 0.3 + progress_ratio(completed, total) * 0.25,
            Self::Patching => 0.58,
            Self::Signing => 0.64,
            Self::Packaging { completed, total } => 0.72 + progress_ratio(completed, total) * 0.24,
            Self::Saving => 0.98,
            Self::Ready => 0.98,
        }
    }

    pub(crate) fn label(self) -> String {
        match self {
            Self::Preparing => "Preparing signing resources".to_string(),
            Self::ResolvingAppId => "Resolving App ID".to_string(),
            Self::DownloadingProfile => "Downloading provisioning profile".to_string(),
            Self::Extracting { completed, total } => {
                format!("Extracting IPA ({completed}/{total})")
            }
            Self::Patching => "Applying app changes".to_string(),
            Self::Signing => "Signing app bundle".to_string(),
            Self::Packaging { completed, total } => {
                format!("Packaging signed IPA ({completed}/{total})")
            }
            Self::Saving => "Saving signed IPA".to_string(),
            Self::Ready => "Signed app ready for transfer".to_string(),
        }
    }
}

impl From<crate::backend::ipa::IpaSigningProgress> for SignIpaProgress {
    fn from(progress: crate::backend::ipa::IpaSigningProgress) -> Self {
        match progress {
            crate::backend::ipa::IpaSigningProgress::Extracting { completed, total } => {
                Self::Extracting { completed, total }
            }
            crate::backend::ipa::IpaSigningProgress::Patching => Self::Patching,
            crate::backend::ipa::IpaSigningProgress::Signing => Self::Signing,
            crate::backend::ipa::IpaSigningProgress::Packaging { completed, total } => {
                Self::Packaging { completed, total }
            }
            crate::backend::ipa::IpaSigningProgress::Saving => Self::Saving,
            crate::backend::ipa::IpaSigningProgress::Ready => Self::Ready,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InstallAppProgress {
    Connecting,
    Uploading {
        transferred_bytes: u64,
        total_bytes: u64,
        completed_files: usize,
        total_files: usize,
    },
    Installing {
        percent: u64,
    },
    Finalizing,
}

impl InstallAppProgress {
    pub(crate) fn progress(self) -> f32 {
        match self {
            Self::Connecting => 0.02,
            Self::Uploading {
                transferred_bytes,
                total_bytes,
                ..
            } => 0.05 + byte_progress_ratio(transferred_bytes, total_bytes) * 0.43,
            Self::Installing { percent } => 0.5 + (percent.min(100) as f32 / 100.) * 0.47,
            Self::Finalizing => 0.99,
        }
    }

    pub(crate) fn label(self) -> String {
        match self {
            Self::Connecting => "Connecting to device".to_string(),
            Self::Uploading {
                transferred_bytes,
                total_bytes,
                completed_files,
                total_files,
            } => format!(
                "Uploading app files ({completed_files}/{total_files}, {} / {})",
                formatted_byte_count(transferred_bytes),
                formatted_byte_count(total_bytes)
            ),
            Self::Installing { percent } => {
                format!("Verifying and installing on device ({percent}%)")
            }
            Self::Finalizing => "Cleaning up device staging".to_string(),
        }
    }
}

impl From<crate::domain::DeviceInstallProgress> for InstallAppProgress {
    fn from(progress: crate::domain::DeviceInstallProgress) -> Self {
        match progress {
            crate::domain::DeviceInstallProgress::Connecting => Self::Connecting,
            crate::domain::DeviceInstallProgress::Uploading {
                transferred_bytes,
                total_bytes,
                completed_files,
                total_files,
            } => Self::Uploading {
                transferred_bytes,
                total_bytes,
                completed_files,
                total_files,
            },
            crate::domain::DeviceInstallProgress::Installing { percent } => {
                Self::Installing { percent }
            }
            crate::domain::DeviceInstallProgress::Finalizing => Self::Finalizing,
        }
    }
}

fn progress_ratio(completed: usize, total: usize) -> f32 {
    if total == 0 {
        1.
    } else {
        (completed as f32 / total as f32).clamp(0., 1.)
    }
}

fn byte_progress_ratio(completed: u64, total: u64) -> f32 {
    if total == 0 {
        1.
    } else {
        (completed as f32 / total as f32).clamp(0., 1.)
    }
}

fn formatted_byte_count(bytes: u64) -> String {
    const MEBIBYTE: f64 = 1024. * 1024.;
    const KIBIBYTE: f64 = 1024.;

    if bytes as f64 >= MEBIBYTE {
        format!("{:.1} MB", bytes as f64 / MEBIBYTE)
    } else if bytes as f64 >= KIBIBYTE {
        format!("{:.1} KB", bytes as f64 / KIBIBYTE)
    } else {
        format!("{bytes} B")
    }
}

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

pub(crate) async fn install_app(
    udid: String,
    signed_app: crate::backend::ipa::SignedAppBundle,
    mut progress: impl FnMut(InstallAppProgress) + Send + 'static,
) -> AppResult<()> {
    crate::backend::device::install_app(udid, signed_app, move |event| {
        progress(event.into());
    })
    .await
    .map_err(AppError::from)
}

pub(crate) async fn sign_ipa(
    request: SignIpaRequest,
    mut progress: impl FnMut(SignIpaProgress) + Send + 'static,
) -> AppResult<SignIpaOutcome> {
    if request.app.bundle_id().trim().is_empty() {
        return Err(AppError::from(
            "The app bundle identifier cannot be empty when signing.",
        ));
    }
    if request.app.name().trim().is_empty() {
        return Err(AppError::from("The app name cannot be empty when signing."));
    }
    progress(SignIpaProgress::Preparing);
    let signing_material = developer_services::load_signing_material(
        &request.certificate_fingerprint,
        &request.public_key_fingerprint,
    )
    .map_err(AppError::from)?;

    progress(SignIpaProgress::ResolvingAppId);
    let targets = signing_app_id_targets(
        &request.team_id,
        &request.team_app_ids,
        request.auto_app_id,
        request.selected_app_id.as_ref(),
        &request.app,
    )?;
    let (app_id, mut updated_account) = resolve_signing_app_id(
        request.developer_context.clone(),
        &request.team_id,
        &request.team_app_ids,
        request.auto_app_id,
        request.selected_app_id.clone(),
        &targets.root.registration_identifier,
        request.app.name(),
    )
    .await?;
    if app_id.developer_id.trim().is_empty() {
        return Err(AppError::from(
            "The selected App ID is missing its Xcode identifier. Refresh Developer Settings, then try again.",
        ));
    }
    if !app_id_matches_identifier(&app_id.identifier, &targets.root.bundle_identifier) {
        return Err(AppError::from(format!(
            "App ID {} does not match bundle identifier {}. Select a matching App ID in Developer Settings.",
            app_id.identifier,
            request.app.bundle_id()
        )));
    }

    let mut team_app_ids = updated_account
        .as_ref()
        .and_then(|account| team_app_ids_for_account(account, &request.team_id))
        .unwrap_or_else(|| request.team_app_ids.clone());
    let effective_bundle_id = targets.root.bundle_identifier;
    let mut bundle_id_replacements = BTreeMap::from([(
        request.app.bundle_id().to_string(),
        effective_bundle_id.clone(),
    )]);
    let mut profile_requests = vec![(effective_bundle_id.clone(), app_id.developer_id)];

    for (nested_bundle, target) in request.app.nested_bundles_for_signing().zip(targets.nested) {
        let (nested_app_id, account_update) = resolve_signing_app_id(
            request.developer_context.clone(),
            &request.team_id,
            &team_app_ids,
            true,
            None,
            &target.registration_identifier,
            &nested_bundle.name,
        )
        .await?;
        if !app_id_matches_identifier(&nested_app_id.identifier, &target.bundle_identifier) {
            return Err(AppError::from(format!(
                "App ID {} does not match nested bundle identifier {}.",
                nested_app_id.identifier, nested_bundle.bundle_id
            )));
        }
        if nested_app_id.developer_id.trim().is_empty() {
            return Err(AppError::from(format!(
                "App ID {} is missing its Xcode identifier.",
                nested_app_id.identifier
            )));
        }
        if let Some(account) = account_update {
            team_app_ids = team_app_ids_for_account(&account, &request.team_id)
                .unwrap_or_else(|| team_app_ids.clone());
            updated_account = Some(account);
        }
        bundle_id_replacements.insert(
            nested_bundle.bundle_id.clone(),
            target.bundle_identifier.clone(),
        );
        profile_requests.push((target.bundle_identifier, nested_app_id.developer_id));
    }

    progress(SignIpaProgress::DownloadingProfile);
    let mut provisioning_profiles = BTreeMap::new();
    for (bundle_id, app_id_id) in profile_requests {
        let profile = download_provisioning_profile(
            request.developer_context.clone(),
            request.team_id.clone(),
            app_id_id,
        )
        .await?;
        provisioning_profiles.insert(bundle_id, profile.bytes);
    }

    let mut metadata = domain_app_metadata(&request.app);
    metadata.bundle_id = effective_bundle_id.clone();
    let destination = match request.output {
        SignIpaOutput::Ipa(path) => crate::backend::ipa::SigningDestination::Ipa(path),
        SignIpaOutput::AppBundle => crate::backend::ipa::SigningDestination::AppBundle,
    };
    let artifact = crate::backend::ipa::sign_ipa(
        crate::backend::ipa::IpaSigningRequest {
            source_path: PathBuf::from(&request.app.path),
            destination,
            metadata,
            bundle_id_replacements,
            icon_override_path: request.app.icon_override_path.as_deref().map(PathBuf::from),
            strip_extensions: request.app.strip_extensions,
            team_id: request.team_id,
            provisioning_profiles,
            private_key_pem: signing_material.private_key_pem,
            certificate_der: signing_material.certificate_der,
        },
        move |event| progress(event.into()),
    )
    .await
    .map_err(AppError::from)?;
    let artifact = match artifact {
        crate::backend::ipa::SigningArtifact::Ipa(path) => SignIpaArtifact::Ipa(path),
        crate::backend::ipa::SigningArtifact::AppBundle(app) => SignIpaArtifact::AppBundle(app),
    };

    Ok(SignIpaOutcome {
        artifact,
        updated_account,
    })
}

pub(crate) fn app_id_provisioning_plan(
    team: &TeamOption,
    auto_app_id: bool,
    selected_app_id: Option<AppIdOption>,
    app: &AppOption,
) -> AppResult<AppIdProvisioningPlan> {
    let targets = signing_app_id_targets(
        &team.identifier,
        &team.app_ids,
        auto_app_id,
        selected_app_id.as_ref(),
        app,
    )?;
    let mut missing = BTreeMap::<String, String>::new();

    if targets.root.existing.is_none() {
        missing.insert(targets.root.registration_identifier, targets.root.name);
    }
    for target in targets.nested {
        if target.existing.is_none() {
            missing
                .entry(target.registration_identifier)
                .or_insert(target.name);
        }
    }

    let app_ids = missing
        .into_iter()
        .map(|(identifier, name)| AppIdProvisioningItem { name, identifier })
        .collect::<Vec<_>>();
    if let Some(available) = team.app_id_available_quantity {
        if app_ids.len() as u64 > available {
            return Err(AppError::from(format!(
                "Signing requires {} new App IDs, but Apple reports only {available} remaining for {}.",
                app_ids.len(),
                team.name
            )));
        }
    }

    Ok(AppIdProvisioningPlan {
        app_ids,
        available_quantity: team.app_id_available_quantity,
    })
}

fn signing_app_id_targets(
    team_id: &str,
    team_app_ids: &[AppIdOption],
    auto_app_id: bool,
    selected_app_id: Option<&AppIdOption>,
    app: &AppOption,
) -> AppResult<SigningAppIdTargets> {
    let fallback_identifier = developer_app_identifier(team_id, app.bundle_id());
    if !auto_app_id {
        let selected = selected_app_id.ok_or_else(|| {
            AppError::from("Select an App ID in Developer Settings before signing.")
        })?;
        if !app_id_matches_bundle(
            &selected.identifier,
            app.bundle_id(),
            &fallback_identifier,
            team_id,
        ) {
            return Err(AppError::from(format!(
                "App ID {} does not match bundle identifier {}. Select a matching App ID in Developer Settings.",
                selected.identifier,
                app.bundle_id()
            )));
        }
        let bundle_identifier = if selected.identifier.ends_with('*') {
            fallback_identifier
        } else {
            selected.identifier.clone()
        };
        let root = SigningAppIdTarget {
            registration_identifier: selected.identifier.clone(),
            bundle_identifier: bundle_identifier.clone(),
            name: app.name().to_string(),
            existing: Some(selected.clone()),
        };
        return Ok(SigningAppIdTargets {
            nested: nested_app_id_targets(team_id, team_app_ids, app, &bundle_identifier),
            root,
        });
    }

    let canonical_bundle_id = identifier_without_team_component(app.bundle_id(), team_id);
    let mut candidates = team_app_ids
        .iter()
        .filter(|app_id| {
            !app_id.identifier.contains('*')
                && identifier_without_team_component(&app_id.identifier, team_id)
                    == canonical_bundle_id
        })
        .map(|app_id| SigningAppIdTarget {
            registration_identifier: app_id.identifier.clone(),
            bundle_identifier: app_id.identifier.clone(),
            name: app.name().to_string(),
            existing: Some(app_id.clone()),
        })
        .collect::<Vec<_>>();
    if !candidates
        .iter()
        .any(|target| target.registration_identifier == fallback_identifier)
    {
        candidates.push(SigningAppIdTarget {
            registration_identifier: fallback_identifier.clone(),
            bundle_identifier: fallback_identifier.clone(),
            name: app.name().to_string(),
            existing: team_app_ids
                .iter()
                .find(|app_id| app_id.identifier == fallback_identifier)
                .cloned(),
        });
    }

    candidates
        .into_iter()
        .map(|root| {
            let nested = nested_app_id_targets(team_id, team_app_ids, app, &root.bundle_identifier);
            let missing = usize::from(root.existing.is_none())
                + nested
                    .iter()
                    .filter(|target| target.existing.is_none())
                    .count();
            let preference =
                if root.existing.is_some() && root.bundle_identifier == app.bundle_id().trim() {
                    0
                } else if root.existing.is_some() && root.bundle_identifier == fallback_identifier {
                    1
                } else if root.existing.is_some() {
                    2
                } else {
                    3
                };
            (missing, preference, SigningAppIdTargets { root, nested })
        })
        .min_by_key(|(missing, preference, _)| (*missing, *preference))
        .map(|(_, _, targets)| targets)
        .ok_or_else(|| AppError::from("Could not determine the App IDs required for signing."))
}

fn nested_app_id_targets(
    team_id: &str,
    team_app_ids: &[AppIdOption],
    app: &AppOption,
    root_bundle_identifier: &str,
) -> Vec<SigningAppIdTarget> {
    app.nested_bundles_for_signing()
        .map(|nested_bundle| {
            let desired_identifier = rebased_nested_bundle_identifier(
                app.original_bundle_id(),
                app.bundle_id(),
                root_bundle_identifier,
                &nested_bundle.bundle_id,
                team_id,
            );
            let canonical_desired = identifier_without_team_component(&desired_identifier, team_id);
            let root_prefix = format!("{root_bundle_identifier}.");
            let existing = team_app_ids
                .iter()
                .find(|app_id| app_id.identifier == desired_identifier)
                .or_else(|| {
                    team_app_ids.iter().find(|app_id| {
                        !app_id.identifier.contains('*')
                            && app_id.identifier.starts_with(&root_prefix)
                            && identifier_without_team_component(&app_id.identifier, team_id)
                                == canonical_desired
                    })
                })
                .cloned();
            let identifier = existing
                .as_ref()
                .map(|app_id| app_id.identifier.clone())
                .unwrap_or(desired_identifier);
            SigningAppIdTarget {
                registration_identifier: identifier.clone(),
                bundle_identifier: identifier,
                name: nested_bundle.name.clone(),
                existing,
            }
        })
        .collect()
}

fn rebased_nested_bundle_identifier(
    original_root_bundle_id: &str,
    current_root_bundle_id: &str,
    effective_root_bundle_id: &str,
    nested_bundle_id: &str,
    team_id: &str,
) -> String {
    let original_root = identifier_without_team_component(original_root_bundle_id, team_id);
    let current_root = identifier_without_team_component(current_root_bundle_id, team_id);
    let nested = identifier_without_team_component(nested_bundle_id, team_id);
    nested
        .strip_prefix(&original_root)
        .or_else(|| nested.strip_prefix(&current_root))
        .filter(|suffix| suffix.starts_with('.'))
        .map(|suffix| format!("{effective_root_bundle_id}{suffix}"))
        .unwrap_or_else(|| {
            effective_nested_bundle_identifier(
                original_root_bundle_id,
                current_root_bundle_id,
                nested_bundle_id,
                team_id,
            )
        })
}

fn identifier_without_team_component(identifier: &str, team_id: &str) -> String {
    let team_id = team_id.trim();
    if team_id.is_empty() {
        return identifier.trim().to_string();
    }
    identifier
        .trim()
        .split('.')
        .filter(|component| *component != team_id)
        .collect::<Vec<_>>()
        .join(".")
}

fn app_id_matches_bundle(
    app_id: &str,
    bundle_id: &str,
    fallback_identifier: &str,
    team_id: &str,
) -> bool {
    app_id_matches_identifier(app_id, bundle_id)
        || app_id_matches_identifier(app_id, fallback_identifier)
        || (!app_id.contains('*')
            && identifier_without_team_component(app_id, team_id)
                == identifier_without_team_component(bundle_id, team_id))
}

fn team_app_ids_for_account(account: &AccountOption, team_id: &str) -> Option<Vec<AppIdOption>> {
    account
        .teams
        .iter()
        .find(|team| team.identifier == team_id)
        .map(|team| team.app_ids.clone())
}

async fn resolve_signing_app_id(
    developer_context: DeveloperSessionContext,
    team_id: &str,
    team_app_ids: &[AppIdOption],
    auto_app_id: bool,
    selected_app_id: Option<AppIdOption>,
    identifier: &str,
    app_name: &str,
) -> AppResult<(AppIdOption, Option<AccountOption>)> {
    if let Some(app_id) =
        existing_signing_app_id(team_app_ids, auto_app_id, selected_app_id, identifier)?
    {
        return Ok((app_id.clone(), None));
    }

    let account = add_app_id(
        developer_context,
        team_id.to_string(),
        identifier.to_string(),
        app_name.to_string(),
    )
    .await?;
    let app_id = account
        .teams
        .iter()
        .find(|team| team.identifier == team_id)
        .and_then(|team| {
            team.app_ids
                .iter()
                .find(|app_id| app_id.identifier == identifier)
        })
        .cloned()
        .ok_or_else(|| {
            AppError::from(format!(
                "Apple created App ID {identifier}, but it was not returned by the developer portal. Refresh Developer Settings and try again."
            ))
        })?;
    Ok((app_id, Some(account)))
}

fn existing_signing_app_id(
    team_app_ids: &[AppIdOption],
    auto_app_id: bool,
    selected_app_id: Option<AppIdOption>,
    automatic_identifier: &str,
) -> AppResult<Option<AppIdOption>> {
    if !auto_app_id {
        return selected_app_id.map(Some).ok_or_else(|| {
            AppError::from("Select an App ID in Developer Settings before signing.")
        });
    }

    Ok(team_app_ids
        .iter()
        .find(|app_id| app_id.identifier == automatic_identifier)
        .cloned())
}

fn developer_app_identifier(team_id: &str, bundle_id: &str) -> String {
    effective_bundle_identifier(bundle_id.trim(), team_id)
}

fn app_id_matches_identifier(app_id: &str, expected_identifier: &str) -> bool {
    app_id == expected_identifier
        || app_id
            .strip_suffix('*')
            .is_some_and(|prefix| expected_identifier.starts_with(prefix))
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

pub(crate) async fn list_developer_devices(
    context: DeveloperSessionContext,
    team_id: String,
) -> AppResult<Vec<DeveloperDeviceOption>> {
    developer_services::list_developer_devices(context.into_backend(), team_id)
        .await
        .map(developer_device_options)
        .map_err(AppError::from)
}

pub(crate) async fn add_developer_device(
    context: DeveloperSessionContext,
    team_id: String,
    name: String,
    udid: String,
) -> AppResult<Vec<DeveloperDeviceOption>> {
    developer_services::add_developer_device(context.into_backend(), team_id, name, udid)
        .await
        .map(developer_device_options)
        .map_err(AppError::from)
}

pub(crate) async fn delete_developer_device(
    context: DeveloperSessionContext,
    team_id: String,
    device_id: String,
) -> AppResult<Vec<DeveloperDeviceOption>> {
    developer_services::delete_developer_device(context.into_backend(), team_id, device_id)
        .await
        .map(developer_device_options)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::models::{
        AppMetadata, EntitlementsSource, NestedBundleKind, NestedBundleOption,
        SupportedDeviceFamily,
    };

    fn app_id(identifier: &str) -> AppIdOption {
        AppIdOption {
            developer_id: format!("xcode-{identifier}"),
            name: identifier.to_string(),
            identifier: identifier.to_string(),
            kind: "Explicit App ID".to_string(),
            capabilities: Vec::new(),
        }
    }

    fn app(bundle_id: &str, nested_bundle_ids: &[&str]) -> AppOption {
        AppOption {
            metadata: AppMetadata::sample(
                "Example",
                bundle_id,
                "1.0",
                "1",
                "Example",
                "15.0",
                vec![SupportedDeviceFamily::IPhone],
            ),
            nested_bundles: nested_bundle_ids
                .iter()
                .map(|bundle_id| NestedBundleOption {
                    name: bundle_id.rsplit('.').next().unwrap().to_string(),
                    bundle_id: (*bundle_id).to_string(),
                    kind: NestedBundleKind::AppExtension,
                })
                .collect(),
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

    fn team(app_ids: Vec<AppIdOption>, available: Option<u64>) -> TeamOption {
        TeamOption {
            name: "Example Team".to_string(),
            identifier: "TEAM".to_string(),
            role: "Admin".to_string(),
            app_id_available_quantity: available,
            app_id_max_quantity: Some(10),
            app_ids,
            certificates: Vec::new(),
        }
    }

    #[test]
    fn automatic_app_id_uses_exact_team_suffixed_match() {
        let app_ids = vec![
            app_id("com.example.other.TEAM"),
            app_id("com.example.app.TEAM"),
        ];

        let selected = existing_signing_app_id(
            &app_ids,
            true,
            Some(app_ids[0].clone()),
            "com.example.app.TEAM",
        )
        .unwrap()
        .unwrap();

        assert_eq!(selected.identifier, "com.example.app.TEAM");
    }

    #[test]
    fn automatic_app_id_requests_creation_when_exact_match_is_missing() {
        let app_ids = vec![app_id("com.example.other.TEAM")];

        assert!(
            existing_signing_app_id(&app_ids, true, None, "com.example.app.TEAM")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn manual_app_id_requires_and_uses_selection() {
        let selected = app_id("com.example.manual.TEAM");
        assert_eq!(
            existing_signing_app_id(&[], false, Some(selected), "ignored")
                .unwrap()
                .unwrap()
                .identifier,
            "com.example.manual.TEAM"
        );
        assert!(existing_signing_app_id(&[], false, None, "ignored").is_err());
    }

    #[test]
    fn developer_identifier_is_suffixed_only_once() {
        assert_eq!(
            developer_app_identifier("TEAM", "com.example.app"),
            "com.example.app.TEAM"
        );
        assert_eq!(
            developer_app_identifier("TEAM", "com.example.app.TEAM"),
            "com.example.app.TEAM"
        );
    }

    #[test]
    fn app_id_matching_accepts_exact_and_wildcard_identifiers() {
        assert!(app_id_matches_identifier(
            "TEAM.com.example.app",
            "TEAM.com.example.app"
        ));
        assert!(app_id_matches_identifier(
            "TEAM.com.example.*",
            "TEAM.com.example.app"
        ));
        assert!(!app_id_matches_identifier(
            "TEAM.com.other.app",
            "TEAM.com.example.app"
        ));
    }

    #[test]
    fn provisioning_plan_counts_unique_missing_root_and_nested_app_ids() {
        let app = app(
            "com.example.app",
            &[
                "com.example.app.widget",
                "com.example.app.widget",
                "com.example.app.share",
            ],
        );
        let team = team(vec![app_id("com.example.app.TEAM.widget")], Some(5));

        let plan = app_id_provisioning_plan(&team, true, None, &app).unwrap();

        assert_eq!(
            plan.app_ids
                .iter()
                .map(|app_id| app_id.identifier.as_str())
                .collect::<Vec<_>>(),
            vec!["com.example.app.TEAM", "com.example.app.TEAM.share"]
        );
        assert_eq!(plan.available_quantity, Some(5));
        assert_eq!(plan.remaining_after_signing(), Some(3));
    }

    #[test]
    fn provisioning_plan_omits_stripped_extensions_but_keeps_nested_apps() {
        let mut app = app(
            "com.example.app",
            &["com.example.app.widget", "com.example.app.watch"],
        );
        app.nested_bundles[1].kind = NestedBundleKind::App;
        app.strip_extensions = true;

        let plan = app_id_provisioning_plan(&team(Vec::new(), Some(5)), true, None, &app).unwrap();

        assert_eq!(
            plan.app_ids
                .iter()
                .map(|app_id| app_id.identifier.as_str())
                .collect::<Vec<_>>(),
            vec!["com.example.app.TEAM", "com.example.app.TEAM.watch"]
        );
    }

    #[test]
    fn provisioning_plan_reuses_unchanged_registered_app_ids() {
        let app = app("com.example.app", &["com.example.app.widget"]);
        let team = team(
            vec![app_id("com.example.app"), app_id("com.example.app.widget")],
            Some(5),
        );

        let targets =
            signing_app_id_targets(&team.identifier, &team.app_ids, true, None, &app).unwrap();
        let plan = app_id_provisioning_plan(&team, true, None, &app).unwrap();

        assert!(plan.app_ids.is_empty());
        assert_eq!(targets.root.bundle_identifier, "com.example.app");
        assert_eq!(
            targets.nested[0].bundle_identifier,
            "com.example.app.widget"
        );
    }

    #[test]
    fn provisioning_plan_reuses_team_prefixed_registered_app_ids() {
        let app = app("com.example.app", &["com.example.app.widget"]);
        let team = team(
            vec![
                app_id("TEAM.com.example.app"),
                app_id("TEAM.com.example.app.widget"),
            ],
            Some(5),
        );

        let targets =
            signing_app_id_targets(&team.identifier, &team.app_ids, true, None, &app).unwrap();
        let plan = app_id_provisioning_plan(&team, true, None, &app).unwrap();

        assert!(plan.app_ids.is_empty());
        assert_eq!(targets.root.bundle_identifier, "TEAM.com.example.app");
        assert_eq!(
            targets.nested[0].bundle_identifier,
            "TEAM.com.example.app.widget"
        );
    }

    #[test]
    fn provisioning_plan_reuses_registered_ids_after_a_root_bundle_override() {
        let mut app = app(
            "com.google.ios.youtube",
            &["com.google.ios.youtube.OpenYouTube.Extension"],
        );
        app.metadata
            .bundle_id
            .set_override("ddcom.google.ios.youtube".to_string());
        let team = team(
            vec![
                app_id("ddcom.google.ios.youtube.TEAM"),
                app_id("ddcom.google.ios.youtube.TEAM.OpenYouTube.Extension"),
            ],
            Some(5),
        );

        let plan = app_id_provisioning_plan(&team, true, None, &app).unwrap();

        assert!(plan.app_ids.is_empty());
    }

    #[test]
    fn provisioning_plan_chooses_the_existing_layout_with_fewer_missing_ids() {
        let app = app("com.example.app", &["com.example.app.widget"]);
        let team = team(
            vec![
                app_id("com.example.app.TEAM"),
                app_id("TEAM.com.example.app"),
                app_id("TEAM.com.example.app.widget"),
            ],
            Some(5),
        );

        let targets =
            signing_app_id_targets(&team.identifier, &team.app_ids, true, None, &app).unwrap();
        let plan = app_id_provisioning_plan(&team, true, None, &app).unwrap();

        assert!(plan.app_ids.is_empty());
        assert_eq!(targets.root.bundle_identifier, "TEAM.com.example.app");
    }

    #[test]
    fn provisioning_plan_rebases_nested_ids_when_root_bundle_id_is_overridden() {
        let mut app = app("com.example.app", &["com.example.app.SubApp.Extension"]);
        app.metadata
            .bundle_id
            .set_override("altcom.example.app".to_string());

        let plan = app_id_provisioning_plan(&team(Vec::new(), Some(5)), true, None, &app).unwrap();

        assert_eq!(
            plan.app_ids
                .iter()
                .map(|app_id| app_id.identifier.as_str())
                .collect::<Vec<_>>(),
            vec![
                "altcom.example.app.TEAM",
                "altcom.example.app.TEAM.SubApp.Extension",
            ]
        );
    }

    #[test]
    fn provisioning_plan_does_not_create_manually_selected_root_app_id() {
        let app = app("com.example.app", &["com.example.app.widget"]);
        let selected = app_id("com.example.app.TEAM");
        let team = team(vec![selected.clone()], Some(4));

        let plan = app_id_provisioning_plan(&team, false, Some(selected), &app).unwrap();

        assert_eq!(plan.app_ids.len(), 1);
        assert_eq!(plan.app_ids[0].identifier, "com.example.app.TEAM.widget");
    }

    #[test]
    fn provisioning_plan_rejects_insufficient_refreshed_quota() {
        let app = app("com.example.app", &["com.example.app.widget"]);
        let error =
            app_id_provisioning_plan(&team(Vec::new(), Some(1)), true, None, &app).unwrap_err();

        assert!(error.user_message().contains("requires 2 new App IDs"));
        assert!(error.user_message().contains("only 1 remaining"));
    }

    #[test]
    fn signing_progress_is_monotonic_across_pipeline_stages() {
        let events = [
            SignIpaProgress::Preparing,
            SignIpaProgress::ResolvingAppId,
            SignIpaProgress::DownloadingProfile,
            SignIpaProgress::Extracting {
                completed: 0,
                total: 10,
            },
            SignIpaProgress::Extracting {
                completed: 10,
                total: 10,
            },
            SignIpaProgress::Patching,
            SignIpaProgress::Signing,
            SignIpaProgress::Packaging {
                completed: 0,
                total: 20,
            },
            SignIpaProgress::Packaging {
                completed: 20,
                total: 20,
            },
            SignIpaProgress::Saving,
            SignIpaProgress::Ready,
        ];
        let values = events.map(SignIpaProgress::progress);

        assert!(values.windows(2).all(|window| window[0] <= window[1]));
        assert_eq!(SignIpaProgress::Saving.progress(), 0.98);
        assert_eq!(
            SignIpaProgress::Extracting {
                completed: 4,
                total: 10
            }
            .label(),
            "Extracting IPA (4/10)"
        );
    }

    #[test]
    fn installation_progress_is_monotonic_and_reports_transfer_size() {
        let events = [
            InstallAppProgress::Connecting,
            InstallAppProgress::Uploading {
                transferred_bytes: 0,
                total_bytes: 10 * 1024 * 1024,
                completed_files: 0,
                total_files: 20,
            },
            InstallAppProgress::Uploading {
                transferred_bytes: 10 * 1024 * 1024,
                total_bytes: 10 * 1024 * 1024,
                completed_files: 20,
                total_files: 20,
            },
            InstallAppProgress::Installing { percent: 0 },
            InstallAppProgress::Installing { percent: 100 },
            InstallAppProgress::Finalizing,
        ];
        let values = events.map(InstallAppProgress::progress);

        assert!(values.windows(2).all(|window| window[0] <= window[1]));
        assert_eq!(InstallAppProgress::Finalizing.progress(), 0.99);
        assert_eq!(
            InstallAppProgress::Uploading {
                transferred_bytes: 5 * 1024 * 1024,
                total_bytes: 10 * 1024 * 1024,
                completed_files: 10,
                total_files: 20,
            }
            .label(),
            "Uploading app files (10/20, 5.0 MB / 10.0 MB)"
        );
    }
}
