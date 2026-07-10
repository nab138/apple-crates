use crate::app::models::{
    AdiBackendKind, AppEntitlement, AppOption, EntitlementValue, MachineIdentity,
    SupportedDeviceFamily,
};
use crate::app::AppResult;
use crate::backend::{preferences as backend_preferences, BackendError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    pub(crate) fn detail(self) -> &'static str {
        match self {
            Self::System => "Follow the operating system appearance.",
            Self::Light => "Always use the light theme.",
            Self::Dark => "Always use the dark theme.",
        }
    }

    pub(crate) fn options() -> [Self; 3] {
        [Self::System, Self::Light, Self::Dark]
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct SideloaderPreferences {
    pub(crate) theme: ThemePreference,
    pub(crate) developer: DeveloperPreferences,
    pub(crate) app: AppPreferences,
    pub(crate) device: Option<DevicePreferences>,
    pub(crate) adi: AdiPreferences,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct DeveloperPreferences {
    pub(crate) account_id: Option<String>,
    pub(crate) team_id: Option<String>,
    pub(crate) certificate_serial_number: Option<String>,
    pub(crate) auto_app_id: bool,
    pub(crate) app_id: Option<String>,
}

impl Default for DeveloperPreferences {
    fn default() -> Self {
        Self {
            account_id: None,
            team_id: None,
            certificate_serial_number: None,
            auto_app_id: true,
            app_id: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct AppPreferences {
    pub(crate) bundle_id: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) overrides: AppOverridePreferences,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct AppOverridePreferences {
    pub(crate) name: Option<String>,
    pub(crate) bundle_id: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) build: Option<String>,
    pub(crate) executable: Option<String>,
    pub(crate) minimum_os: Option<String>,
    pub(crate) supported_devices: Option<String>,
    pub(crate) icon_path: Option<String>,
    pub(crate) entitlements: Option<Vec<EntitlementPreferences>>,
    #[serde(default)]
    pub(crate) strip_extensions: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct EntitlementPreferences {
    pub(crate) key: String,
    pub(crate) value_type: String,
    pub(crate) value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) values: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DevicePreferences {
    pub(crate) udid: String,
    pub(crate) connection: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct AdiPreferences {
    pub(crate) backend: Option<StoredAdiBackendKind>,
    pub(crate) machine: MachineIdentityPreferences,
    pub(crate) android_adi_identifier: Option<String>,
    pub(crate) android_device: MachineIdentityPreferences,
    pub(crate) android_device_uuid: Option<String>,
    pub(crate) android_device_identity_customized: bool,
    #[serde(skip_serializing)]
    pub(crate) android_machine: MachineIdentityPreferences,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct MachineIdentityPreferences {
    pub(crate) machine_name: Option<String>,
    pub(crate) os_name: Option<String>,
    pub(crate) os_version: Option<String>,
    pub(crate) machine_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StoredAdiBackendKind {
    SystemAdid,
    WindowsCoreAdi,
    AndroidCoreAdi,
}

impl From<AdiBackendKind> for StoredAdiBackendKind {
    fn from(kind: AdiBackendKind) -> Self {
        match kind {
            AdiBackendKind::SystemAdid => Self::SystemAdid,
            AdiBackendKind::WindowsCoreAdi => Self::WindowsCoreAdi,
            AdiBackendKind::AndroidCoreAdi => Self::AndroidCoreAdi,
        }
    }
}

impl From<StoredAdiBackendKind> for AdiBackendKind {
    fn from(kind: StoredAdiBackendKind) -> Self {
        match kind {
            StoredAdiBackendKind::SystemAdid => Self::SystemAdid,
            StoredAdiBackendKind::WindowsCoreAdi => Self::WindowsCoreAdi,
            StoredAdiBackendKind::AndroidCoreAdi => Self::AndroidCoreAdi,
        }
    }
}

impl From<&MachineIdentity> for MachineIdentityPreferences {
    fn from(identity: &MachineIdentity) -> Self {
        Self {
            machine_name: Some(identity.machine_name.to_string()),
            os_name: Some(identity.os_name.to_string()),
            os_version: Some(identity.os_version.to_string()),
            machine_id: Some(identity.machine_id.to_string()),
        }
    }
}

impl From<&AppEntitlement> for EntitlementPreferences {
    fn from(entitlement: &AppEntitlement) -> Self {
        Self {
            key: entitlement.key.to_string(),
            value_type: entitlement.value.type_label().to_string(),
            value: entitlement.value.edit_text(),
            values: matches!(entitlement.value, EntitlementValue::Array(_))
                .then(|| entitlement.value.array_edit_values()),
        }
    }
}

impl From<&EntitlementPreferences> for AppEntitlement {
    fn from(entitlement: &EntitlementPreferences) -> Self {
        let value = if entitlement.value_type == "Array" {
            entitlement
                .values
                .clone()
                .map(EntitlementValue::string_array)
                .unwrap_or_else(|| {
                    EntitlementValue::from_type_and_text(
                        &entitlement.value_type,
                        &entitlement.value,
                    )
                })
        } else {
            EntitlementValue::from_type_and_text(&entitlement.value_type, &entitlement.value)
        };

        Self {
            key: entitlement.key.clone(),
            value,
        }
    }
}

impl From<&AppOption> for AppOverridePreferences {
    fn from(app: &AppOption) -> Self {
        Self {
            name: app
                .metadata
                .name
                .override_value
                .as_ref()
                .map(ToString::to_string),
            bundle_id: app
                .metadata
                .bundle_id
                .override_value
                .as_ref()
                .map(ToString::to_string),
            version: app
                .metadata
                .version
                .override_value
                .as_ref()
                .map(ToString::to_string),
            build: app
                .metadata
                .build
                .override_value
                .as_ref()
                .map(ToString::to_string),
            executable: app
                .metadata
                .executable
                .override_value
                .as_ref()
                .map(ToString::to_string),
            minimum_os: app
                .metadata
                .minimum_os
                .override_value
                .as_ref()
                .map(ToString::to_string),
            supported_devices: app
                .metadata
                .supported_devices
                .override_value
                .as_ref()
                .map(|devices| SupportedDeviceFamily::display_list(devices).to_string()),
            icon_path: app.icon_override_path.as_ref().map(ToString::to_string),
            entitlements: app.entitlement_overrides.as_ref().map(|entitlements| {
                entitlements
                    .iter()
                    .map(EntitlementPreferences::from)
                    .collect()
            }),
            strip_extensions: app.strip_extensions,
        }
    }
}

pub(crate) fn apply_app_overrides(app: &mut AppOption, overrides: &AppOverridePreferences) {
    app.metadata.name.override_value = overrides.name.clone();
    app.metadata.bundle_id.override_value = overrides.bundle_id.clone();
    app.metadata.version.override_value = overrides.version.clone();
    app.metadata.build.override_value = overrides.build.clone();
    app.metadata.executable.override_value = overrides.executable.clone();
    app.metadata.minimum_os.override_value = overrides.minimum_os.clone();
    app.metadata.supported_devices.override_value = overrides
        .supported_devices
        .as_ref()
        .map(|devices| SupportedDeviceFamily::parse_list(devices));
    app.icon_override_path = overrides.icon_path.clone();
    app.entitlement_overrides = overrides.entitlements.as_ref().map(|entitlements| {
        entitlements
            .iter()
            .map(AppEntitlement::from)
            .collect::<Vec<_>>()
    });
    app.strip_extensions = overrides.strip_extensions && app.app_extension_count() > 0;
}

pub(crate) fn load_preferences() -> AppResult<SideloaderPreferences> {
    let Some(contents) = backend_preferences::load_settings_toml()? else {
        return Ok(SideloaderPreferences::default());
    };

    toml::from_str(&contents).map_err(|error| {
        BackendError::Preferences(format!("Failed to parse settings: {error}")).into()
    })
}

pub(crate) fn save_preferences(preferences: &SideloaderPreferences) -> AppResult<()> {
    let contents = toml::to_string_pretty(preferences).map_err(|error| {
        BackendError::Preferences(format!("Failed to encode settings: {error}"))
    })?;
    backend_preferences::save_settings_toml(&contents).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::models::{
        AppMetadata, EntitlementsSource, NestedBundleKind, NestedBundleOption, PatchOption,
    };

    fn sample_app() -> AppOption {
        AppOption {
            metadata: AppMetadata::sample(
                "Original",
                "com.example.original",
                "1.0",
                "1",
                "Original",
                "16.0",
                vec![SupportedDeviceFamily::IPhone],
            ),
            nested_bundles: vec![NestedBundleOption {
                name: "Widget".to_string(),
                bundle_id: "com.example.original.widget".to_string(),
                kind: NestedBundleKind::AppExtension,
            }],
            strip_extensions: false,
            path: "/tmp/App.ipa".to_string(),
            icon_path: None,
            icon_override_path: None,
            entitlements: Vec::new(),
            entitlements_source: EntitlementsSource::GeneratedFallback,
            entitlement_overrides: None,
            patches: vec![PatchOption {
                name: "Patch".to_string(),
                detail: "Detail".to_string(),
            }],
        }
    }

    #[test]
    fn app_overrides_restore_metadata_icon_and_entitlements() {
        let mut app = sample_app();
        apply_app_overrides(
            &mut app,
            &AppOverridePreferences {
                name: Some("Restored".to_string()),
                bundle_id: Some("com.example.restored".to_string()),
                version: Some("2.0".to_string()),
                build: Some("42".to_string()),
                executable: Some("RestoredExecutable".to_string()),
                minimum_os: Some("17.0".to_string()),
                supported_devices: Some("iPhone, iPad".to_string()),
                icon_path: Some("/tmp/icon.png".to_string()),
                entitlements: Some(vec![EntitlementPreferences {
                    key: "get-task-allow".to_string(),
                    value_type: "Boolean".to_string(),
                    value: "true".to_string(),
                    values: None,
                }]),
                strip_extensions: true,
            },
        );

        assert_eq!(app.name(), "Restored");
        assert_eq!(app.bundle_id(), "com.example.restored");
        assert_eq!(app.version(), "2.0");
        assert_eq!(app.build(), "42");
        assert_eq!(app.metadata.executable.value(), "RestoredExecutable");
        assert_eq!(app.metadata.minimum_os.value(), "17.0");
        assert_eq!(
            app.metadata.supported_devices.value(),
            &[SupportedDeviceFamily::IPhone, SupportedDeviceFamily::IPad]
        );
        assert_eq!(app.icon_override_path.as_deref(), Some("/tmp/icon.png"));
        assert!(app.strip_extensions);
        assert!(AppOverridePreferences::from(&app).strip_extensions);
        assert_eq!(
            app.entitlement_overrides,
            Some(vec![AppEntitlement {
                key: "get-task-allow".to_string(),
                value: EntitlementValue::Boolean(true),
            }])
        );
    }

    #[test]
    fn older_app_overrides_default_to_preserving_extensions() {
        let overrides = toml::from_str::<AppOverridePreferences>("").unwrap();

        assert!(!overrides.strip_extensions);
    }
}
