use crate::app::models::{
    AccountOption, AdiBackendAvailability, AdiBackendDetail, AdiBackendKind, AdiBackendOption,
    AdiProvisioningState, AdiRepairAction, AppEntitlement, AppIdCapabilityOption, AppIdOption,
    AppMetadata, AppOption, DeveloperDeviceOption, DevelopmentCertificateOption, DeviceOption,
    EntitlementValue, EntitlementsSource, MachineIdentity, NestedBundleKind, NestedBundleOption,
    PatchOption, SupportedDeviceFamily, TeamOption,
};
use crate::domain::{
    adi as domain_adi, device as domain_device, identity as domain_identity, ipa as domain_ipa,
    DeveloperAccount, DeveloperAppId, DeveloperAppIdCapability, DeveloperCertificate,
    DeveloperDevice, DeveloperTeam,
};

pub(crate) fn account_option(account: DeveloperAccount) -> AccountOption {
    let profile_name = account
        .profile_name
        .clone()
        .unwrap_or_else(|| account.email.clone());
    let status = account
        .token_expires_at
        .map(|expires_at| format!("Token expires {expires_at}"))
        .unwrap_or_else(|| "Cached".to_string());
    let detail = account
        .last_refreshed_at
        .map(|refreshed_at| format!("Last refreshed {refreshed_at}"))
        .unwrap_or_else(|| "Loaded from account cache".to_string());

    AccountOption {
        id: account.id,
        label: profile_name,
        apple_id: account.email,
        detail,
        status,
        teams: account.teams.into_iter().map(team_option).collect(),
    }
}

pub(crate) fn account_options(accounts: Vec<DeveloperAccount>) -> Vec<AccountOption> {
    accounts.into_iter().map(account_option).collect()
}

pub(crate) fn domain_machine_identity(
    identity: &MachineIdentity,
) -> domain_identity::MachineIdentity {
    domain_identity::MachineIdentity {
        machine_name: identity.machine_name.clone(),
        os_name: identity.os_name.clone(),
        os_version: identity.os_version.clone(),
        machine_id: identity.machine_id.clone(),
    }
}

pub(crate) fn machine_identity(identity: domain_identity::MachineIdentity) -> MachineIdentity {
    MachineIdentity {
        machine_name: identity.machine_name,
        os_name: identity.os_name,
        os_version: identity.os_version,
        machine_id: identity.machine_id,
    }
}

pub(crate) fn domain_adi_kind(kind: AdiBackendKind) -> domain_adi::AdiBackendKind {
    match kind {
        AdiBackendKind::SystemAdid => domain_adi::AdiBackendKind::SystemAdid,
        AdiBackendKind::WindowsCoreAdi => domain_adi::AdiBackendKind::WindowsCoreAdi,
        AdiBackendKind::AndroidCoreAdi => domain_adi::AdiBackendKind::AndroidCoreAdi,
    }
}

pub(crate) fn adi_kind(kind: domain_adi::AdiBackendKind) -> AdiBackendKind {
    match kind {
        domain_adi::AdiBackendKind::SystemAdid => AdiBackendKind::SystemAdid,
        domain_adi::AdiBackendKind::WindowsCoreAdi => AdiBackendKind::WindowsCoreAdi,
        domain_adi::AdiBackendKind::AndroidCoreAdi => AdiBackendKind::AndroidCoreAdi,
    }
}

pub(crate) fn adi_backends(backends: Vec<domain_adi::AdiBackend>) -> Vec<AdiBackendOption> {
    backends.into_iter().map(adi_backend).collect()
}

pub(crate) fn app_option(app: domain_ipa::IpaApp) -> AppOption {
    AppOption {
        metadata: AppMetadata::sample(
            app.metadata.name,
            app.metadata.bundle_id,
            app.metadata.version,
            app.metadata.build,
            app.metadata.executable,
            app.metadata.minimum_os,
            app.metadata
                .supported_devices
                .into_iter()
                .map(supported_device_family)
                .collect(),
        ),
        nested_bundles: app
            .nested_bundles
            .into_iter()
            .map(|bundle| NestedBundleOption {
                name: bundle.name,
                bundle_id: bundle.bundle_id,
                kind: match bundle.kind {
                    domain_ipa::NestedBundleKind::App => NestedBundleKind::App,
                    domain_ipa::NestedBundleKind::AppExtension => NestedBundleKind::AppExtension,
                },
            })
            .collect(),
        strip_extensions: false,
        path: app.path,
        icon_path: app.icon_path,
        icon_override_path: None,
        entitlements: app.entitlements.into_iter().map(app_entitlement).collect(),
        entitlements_source: entitlements_source(app.entitlements_source),
        entitlement_overrides: None,
        patches: app
            .patches
            .into_iter()
            .map(|patch| PatchOption {
                name: patch.name,
                detail: patch.detail,
            })
            .collect(),
    }
}

pub(crate) fn device_options(devices: Vec<domain_device::Device>) -> Vec<DeviceOption> {
    devices.into_iter().map(device_option).collect()
}

pub(crate) fn developer_device_options(
    devices: Vec<DeveloperDevice>,
) -> Vec<DeveloperDeviceOption> {
    devices
        .into_iter()
        .map(|device| DeveloperDeviceOption {
            id: device.id,
            name: device.name,
            udid: device.udid,
        })
        .collect()
}

pub(crate) fn domain_patch(patch: PatchOption) -> domain_ipa::Patch {
    domain_ipa::Patch {
        name: patch.name,
        detail: patch.detail,
    }
}

pub(crate) fn domain_app_metadata(app: &AppOption) -> domain_ipa::AppMetadata {
    domain_ipa::AppMetadata {
        name: app.name().to_string(),
        bundle_id: app.bundle_id().to_string(),
        version: app.version().to_string(),
        build: app.build().to_string(),
        executable: app.metadata.executable.value().to_string(),
        minimum_os: app.metadata.minimum_os.value().to_string(),
        supported_devices: app
            .supported_devices()
            .iter()
            .copied()
            .map(domain_supported_device_family)
            .collect(),
    }
}

pub(crate) fn domain_app_entitlements(
    entitlements: &[AppEntitlement],
) -> Vec<domain_ipa::AppEntitlement> {
    entitlements
        .iter()
        .map(|entitlement| domain_ipa::AppEntitlement {
            key: entitlement.key.clone(),
            value: domain_entitlement_value(&entitlement.value),
        })
        .collect()
}

fn team_option(team: DeveloperTeam) -> TeamOption {
    TeamOption {
        name: team.name,
        identifier: team.id,
        role: team.role,
        app_id_available_quantity: team.app_id_available_quantity,
        app_id_max_quantity: team.app_id_max_quantity,
        app_ids: team.app_ids.into_iter().map(app_id_option).collect(),
        certificates: team
            .certificates
            .into_iter()
            .map(certificate_option)
            .collect(),
    }
}

fn app_id_option(app_id: DeveloperAppId) -> AppIdOption {
    AppIdOption {
        developer_id: if app_id.developer_id.is_empty() {
            app_id.id.clone()
        } else {
            app_id.developer_id
        },
        name: app_id.name,
        identifier: app_id.id,
        kind: app_id.kind,
        capabilities: app_id
            .capabilities
            .into_iter()
            .map(capability_option)
            .collect(),
    }
}

fn capability_option(capability: DeveloperAppIdCapability) -> AppIdCapabilityOption {
    AppIdCapabilityOption {
        key: capability.key,
        label: capability.label,
        detail: capability.detail,
        enabled: capability.enabled,
    }
}

fn certificate_option(certificate: DeveloperCertificate) -> DevelopmentCertificateOption {
    DevelopmentCertificateOption {
        id: certificate.id,
        name: certificate.name,
        serial_number: certificate.serial_number,
        machine_name: certificate.machine_name,
        private_key_available: certificate.private_key_available,
        certificate_fingerprint: certificate.certificate_fingerprint,
        public_key_fingerprint: certificate.public_key_fingerprint,
    }
}

fn adi_backend(backend: domain_adi::AdiBackend) -> AdiBackendOption {
    AdiBackendOption {
        kind: adi_kind(backend.kind),
        name: backend.name,
        detail: backend.detail,
        availability: adi_availability(backend.availability),
        details: backend
            .details
            .into_iter()
            .map(|detail| AdiBackendDetail {
                label: detail.label,
                value: detail.value,
            })
            .collect(),
        provisioning_state: adi_provisioning_state(backend.provisioning_state),
        editable_identity: backend.editable_identity,
        repair_action: backend.repair_action.map(adi_repair_action),
    }
}

fn adi_availability(availability: domain_adi::AdiBackendAvailability) -> AdiBackendAvailability {
    match availability {
        domain_adi::AdiBackendAvailability::Ready => AdiBackendAvailability::Ready,
        domain_adi::AdiBackendAvailability::NeedsSetup => AdiBackendAvailability::NeedsSetup,
        domain_adi::AdiBackendAvailability::Unavailable => AdiBackendAvailability::Unavailable,
    }
}

fn adi_provisioning_state(state: domain_adi::AdiProvisioningState) -> AdiProvisioningState {
    match state {
        domain_adi::AdiProvisioningState::Unknown => AdiProvisioningState::Unknown,
        domain_adi::AdiProvisioningState::NotAvailable => AdiProvisioningState::NotAvailable,
        domain_adi::AdiProvisioningState::Provisioned => AdiProvisioningState::Provisioned,
        domain_adi::AdiProvisioningState::NotProvisioned => AdiProvisioningState::NotProvisioned,
        domain_adi::AdiProvisioningState::Error(error) => AdiProvisioningState::Error(error),
    }
}

fn adi_repair_action(action: domain_adi::AdiRepairAction) -> AdiRepairAction {
    match action {
        domain_adi::AdiRepairAction::InstallCoreAdi => AdiRepairAction::InstallCoreAdi,
        domain_adi::AdiRepairAction::LocateLibrary => AdiRepairAction::LocateLibrary,
    }
}

fn supported_device_family(family: domain_ipa::SupportedDeviceFamily) -> SupportedDeviceFamily {
    match family {
        domain_ipa::SupportedDeviceFamily::IPhone => SupportedDeviceFamily::IPhone,
        domain_ipa::SupportedDeviceFamily::IPad => SupportedDeviceFamily::IPad,
        domain_ipa::SupportedDeviceFamily::AppleTv => SupportedDeviceFamily::AppleTv,
        domain_ipa::SupportedDeviceFamily::AppleWatch => SupportedDeviceFamily::AppleWatch,
        domain_ipa::SupportedDeviceFamily::Mac => SupportedDeviceFamily::Mac,
    }
}

fn domain_supported_device_family(
    family: SupportedDeviceFamily,
) -> domain_ipa::SupportedDeviceFamily {
    match family {
        SupportedDeviceFamily::IPhone => domain_ipa::SupportedDeviceFamily::IPhone,
        SupportedDeviceFamily::IPad => domain_ipa::SupportedDeviceFamily::IPad,
        SupportedDeviceFamily::AppleTv => domain_ipa::SupportedDeviceFamily::AppleTv,
        SupportedDeviceFamily::AppleWatch => domain_ipa::SupportedDeviceFamily::AppleWatch,
        SupportedDeviceFamily::Mac => domain_ipa::SupportedDeviceFamily::Mac,
    }
}

fn app_entitlement(entitlement: domain_ipa::AppEntitlement) -> AppEntitlement {
    AppEntitlement {
        key: entitlement.key,
        value: entitlement_value(entitlement.value),
    }
}

fn entitlement_value(value: domain_ipa::EntitlementValue) -> EntitlementValue {
    match value {
        domain_ipa::EntitlementValue::String(value) => EntitlementValue::String(value),
        domain_ipa::EntitlementValue::Boolean(value) => EntitlementValue::Boolean(value),
        domain_ipa::EntitlementValue::Integer(value) => EntitlementValue::Integer(value),
        domain_ipa::EntitlementValue::Number(value) => EntitlementValue::Number(value),
        domain_ipa::EntitlementValue::Array(values) => {
            EntitlementValue::Array(values.into_iter().map(entitlement_value).collect())
        }
        domain_ipa::EntitlementValue::Dictionary(values) => EntitlementValue::Dictionary(
            values
                .into_iter()
                .map(|(key, value)| (key, entitlement_value(value)))
                .collect(),
        ),
        domain_ipa::EntitlementValue::Data(value) => EntitlementValue::Data(value),
        domain_ipa::EntitlementValue::Date(value) => EntitlementValue::Date(value),
        domain_ipa::EntitlementValue::Uid(value) => EntitlementValue::Uid(value),
        domain_ipa::EntitlementValue::Unknown(value) => EntitlementValue::Unknown(value),
    }
}

fn domain_entitlement_value(value: &EntitlementValue) -> domain_ipa::EntitlementValue {
    match value {
        EntitlementValue::String(value) => domain_ipa::EntitlementValue::String(value.clone()),
        EntitlementValue::Boolean(value) => domain_ipa::EntitlementValue::Boolean(*value),
        EntitlementValue::Integer(value) => domain_ipa::EntitlementValue::Integer(*value),
        EntitlementValue::Number(value) => domain_ipa::EntitlementValue::Number(*value),
        EntitlementValue::Array(values) => domain_ipa::EntitlementValue::Array(
            values.iter().map(domain_entitlement_value).collect(),
        ),
        EntitlementValue::Dictionary(values) => domain_ipa::EntitlementValue::Dictionary(
            values
                .iter()
                .map(|(key, value)| (key.clone(), domain_entitlement_value(value)))
                .collect(),
        ),
        EntitlementValue::Data(value) => domain_ipa::EntitlementValue::Data(value.clone()),
        EntitlementValue::Date(value) => domain_ipa::EntitlementValue::Date(value.clone()),
        EntitlementValue::Uid(value) => domain_ipa::EntitlementValue::Uid(*value),
        EntitlementValue::Unknown(value) => domain_ipa::EntitlementValue::Unknown(value.clone()),
    }
}

fn entitlements_source(source: domain_ipa::EntitlementsSource) -> EntitlementsSource {
    match source {
        domain_ipa::EntitlementsSource::Embedded => EntitlementsSource::Embedded,
        domain_ipa::EntitlementsSource::GeneratedFallback => EntitlementsSource::GeneratedFallback,
    }
}

fn device_option(device: domain_device::Device) -> DeviceOption {
    DeviceOption {
        name: device.name,
        model: device.model,
        os: device.os,
        udid: device.udid,
        connection: device.connection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developer_account_maps_to_ui_account_option_at_app_boundary() {
        let account = DeveloperAccount {
            id: "account-1".to_string(),
            email: "user@example.com".to_string(),
            profile_name: Some("User".to_string()),
            token_expires_at: Some("tomorrow".to_string()),
            token_expires_at_epoch_millis: Some(42),
            last_refreshed_at: Some("today".to_string()),
            teams: vec![DeveloperTeam {
                id: "TEAM1".to_string(),
                name: "Team One".to_string(),
                role: "Admin".to_string(),
                app_id_available_quantity: Some(4),
                app_id_max_quantity: Some(10),
                app_ids: vec![DeveloperAppId {
                    id: "TEAM1.com.example.app".to_string(),
                    developer_id: String::new(),
                    name: "Example".to_string(),
                    kind: "Explicit".to_string(),
                    capabilities: vec![DeveloperAppIdCapability {
                        key: "icloud".to_string(),
                        label: "iCloud".to_string(),
                        detail: "Cloud documents".to_string(),
                        enabled: true,
                    }],
                }],
                certificates: vec![DeveloperCertificate {
                    id: "certificate-1".to_string(),
                    name: "Development".to_string(),
                    serial_number: "SERIAL".to_string(),
                    machine_name: "Mac".to_string(),
                    private_key_available: true,
                    certificate_fingerprint: Some("certificate-fingerprint".to_string()),
                    public_key_fingerprint: Some("fingerprint".to_string()),
                }],
            }],
        };

        let option = account_option(account);

        assert_eq!(option.id, "account-1");
        assert_eq!(option.label, "User");
        assert_eq!(option.apple_id, "user@example.com");
        assert_eq!(option.status, "Token expires tomorrow");
        assert_eq!(option.detail, "Last refreshed today");
        assert_eq!(option.teams[0].identifier, "TEAM1");
        assert_eq!(
            option.teams[0].app_ids[0].developer_id,
            "TEAM1.com.example.app"
        );
        assert!(option.teams[0].app_ids[0].capabilities[0].enabled);
        assert!(option.teams[0].certificates[0].private_key_available);
    }

    #[test]
    fn adi_backend_maps_to_ui_option_at_app_boundary() {
        let options = adi_backends(vec![domain_adi::AdiBackend {
            kind: domain_adi::AdiBackendKind::AndroidCoreAdi,
            name: "Android CoreADI".to_string(),
            detail: "Apple Music".to_string(),
            availability: domain_adi::AdiBackendAvailability::NeedsSetup,
            details: vec![domain_adi::AdiBackendDetail {
                label: "Library".to_string(),
                value: "Missing".to_string(),
            }],
            provisioning_state: domain_adi::AdiProvisioningState::Error("No ADI".to_string()),
            editable_identity: true,
            repair_action: Some(domain_adi::AdiRepairAction::InstallCoreAdi),
        }]);

        let option = &options[0];
        assert_eq!(option.kind, AdiBackendKind::AndroidCoreAdi);
        assert_eq!(option.name, "Android CoreADI");
        assert_eq!(option.availability, AdiBackendAvailability::NeedsSetup);
        assert_eq!(option.details[0].label, "Library");
        assert!(matches!(
            option.provisioning_state,
            AdiProvisioningState::Error(ref error) if error == "No ADI"
        ));
        assert_eq!(option.repair_action, Some(AdiRepairAction::InstallCoreAdi));
    }

    #[test]
    fn device_maps_to_ui_option_at_app_boundary() {
        let options = device_options(vec![domain_device::Device {
            name: "iPhone".to_string(),
            model: "iPhone15,3".to_string(),
            os: "iOS 18".to_string(),
            udid: "UDID".to_string(),
            connection: "USB".to_string(),
        }]);

        assert_eq!(options[0].name, "iPhone");
        assert_eq!(options[0].model, "iPhone15,3");
        assert_eq!(options[0].connection, "USB");
    }

    #[test]
    fn developer_device_maps_to_settings_option() {
        let options = developer_device_options(vec![DeveloperDevice {
            id: "portal-device-id".to_string(),
            name: "Development iPhone".to_string(),
            udid: "0000000000000000000000000000000000000000".to_string(),
        }]);

        assert_eq!(options[0].id, "portal-device-id");
        assert_eq!(options[0].name, "Development iPhone");
        assert_eq!(options[0].udid.len(), 40);
    }

    #[test]
    fn ipa_app_maps_to_ui_app_option_at_app_boundary() {
        let option = app_option(domain_ipa::IpaApp {
            metadata: domain_ipa::AppMetadata {
                name: "Example".to_string(),
                bundle_id: "com.example.app".to_string(),
                version: "1.0".to_string(),
                build: "42".to_string(),
                executable: "Example".to_string(),
                minimum_os: "16.0".to_string(),
                supported_devices: vec![
                    domain_ipa::SupportedDeviceFamily::IPhone,
                    domain_ipa::SupportedDeviceFamily::IPad,
                ],
            },
            nested_bundles: vec![domain_ipa::NestedBundle {
                name: "Widget".to_string(),
                bundle_id: "com.example.app.Widget".to_string(),
                kind: domain_ipa::NestedBundleKind::AppExtension,
            }],
            path: "/tmp/Example.ipa".to_string(),
            icon_path: Some("/tmp/icon.png".to_string()),
            entitlements: vec![domain_ipa::AppEntitlement {
                key: "get-task-allow".to_string(),
                value: domain_ipa::EntitlementValue::Boolean(true),
            }],
            entitlements_source: domain_ipa::EntitlementsSource::Embedded,
            patches: vec![domain_ipa::Patch {
                name: "Patch".to_string(),
                detail: "Detail".to_string(),
            }],
        });

        assert_eq!(option.name(), "Example");
        assert_eq!(option.bundle_id(), "com.example.app");
        assert_eq!(option.nested_bundles[0].bundle_id, "com.example.app.Widget");
        assert_eq!(
            option.nested_bundles[0].kind,
            NestedBundleKind::AppExtension
        );
        assert_eq!(
            option.supported_devices(),
            &[SupportedDeviceFamily::IPhone, SupportedDeviceFamily::IPad]
        );
        assert_eq!(option.icon_path.as_deref(), Some("/tmp/icon.png"));
        assert_eq!(option.entitlements[0].key, "get-task-allow");
        assert_eq!(option.entitlements_source, EntitlementsSource::Embedded);
        assert_eq!(option.patches[0].name, "Patch");
    }
}
