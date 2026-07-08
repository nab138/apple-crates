use crate::models::{
    AdiBackendKind, AppEntitlement, AppOption, EntitlementValue, MachineIdentity,
    SupportedDeviceFamily,
};
use crate::paths::app_data_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

const SETTINGS_FILE: &str = "settings.toml";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct SideloaderPreferences {
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
    pub(crate) auto_app_id: bool,
    pub(crate) app_id: Option<String>,
}

impl Default for DeveloperPreferences {
    fn default() -> Self {
        Self {
            account_id: None,
            team_id: None,
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
            key: entitlement.key.clone().into(),
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
        }
    }
}

pub(crate) fn apply_app_overrides(app: &mut AppOption, overrides: &AppOverridePreferences) {
    app.metadata.name.override_value = overrides.name.clone().map(Into::into);
    app.metadata.bundle_id.override_value = overrides.bundle_id.clone().map(Into::into);
    app.metadata.version.override_value = overrides.version.clone().map(Into::into);
    app.metadata.build.override_value = overrides.build.clone().map(Into::into);
    app.metadata.executable.override_value = overrides.executable.clone().map(Into::into);
    app.metadata.minimum_os.override_value = overrides.minimum_os.clone().map(Into::into);
    app.metadata.supported_devices.override_value = overrides
        .supported_devices
        .as_ref()
        .map(|devices| SupportedDeviceFamily::parse_list(devices));
    app.icon_override_path = overrides.icon_path.clone().map(Into::into);
    app.entitlement_overrides = overrides.entitlements.as_ref().map(|entitlements| {
        entitlements
            .iter()
            .map(AppEntitlement::from)
            .collect::<Vec<_>>()
    });
}

pub(crate) fn load_preferences() -> Result<SideloaderPreferences, String> {
    let path = settings_path()?;
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(SideloaderPreferences::default());
        }
        Err(error) => {
            return Err(format!(
                "Failed to read settings at {}: {error}",
                path.display()
            ));
        }
    };

    toml::from_str(&contents)
        .map_err(|error| format!("Failed to parse settings at {}: {error}", path.display()))
}

pub(crate) fn save_preferences(preferences: &SideloaderPreferences) -> Result<(), String> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create settings folder at {}: {error}",
                parent.display()
            )
        })?;
    }

    let contents = toml::to_string_pretty(preferences)
        .map_err(|error| format!("Failed to encode settings: {error}"))?;
    fs::write(&path, contents)
        .map_err(|error| format!("Failed to write settings at {}: {error}", path.display()))
}

fn settings_path() -> Result<PathBuf, String> {
    app_data_dir()
        .map(|path| path.join(SETTINGS_FILE))
        .ok_or_else(|| "The application data folder is not available.".to_string())
}
