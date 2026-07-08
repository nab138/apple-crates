use gpui::SharedString;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PickerId {
    Account,
    Device,
}

#[derive(Clone, Debug)]
pub(crate) struct AppIdOption {
    pub(crate) name: SharedString,
    pub(crate) identifier: SharedString,
    pub(crate) kind: SharedString,
}

#[derive(Clone, Debug)]
pub(crate) struct TeamOption {
    pub(crate) name: SharedString,
    pub(crate) identifier: SharedString,
    pub(crate) role: SharedString,
    pub(crate) app_ids: Vec<AppIdOption>,
}

#[derive(Clone, Debug)]
pub(crate) struct AccountOption {
    pub(crate) id: SharedString,
    pub(crate) label: SharedString,
    pub(crate) apple_id: SharedString,
    pub(crate) detail: SharedString,
    pub(crate) status: SharedString,
    pub(crate) teams: Vec<TeamOption>,
}

#[derive(Clone, Debug)]
pub(crate) struct PatchOption {
    pub(crate) name: SharedString,
    pub(crate) detail: SharedString,
}

#[derive(Clone, Debug)]
pub(crate) struct AppFieldValue {
    pub(crate) default: SharedString,
    pub(crate) override_value: Option<SharedString>,
}

impl AppFieldValue {
    pub(crate) fn new(default: impl Into<SharedString>) -> Self {
        Self {
            default: default.into(),
            override_value: None,
        }
    }

    pub(crate) fn value(&self) -> &SharedString {
        self.override_value.as_ref().unwrap_or(&self.default)
    }

    pub(crate) fn is_overridden(&self) -> bool {
        self.override_value.is_some()
    }

    pub(crate) fn set_override(&mut self, value: String) {
        if value == self.default.as_ref() {
            self.override_value = None;
        } else {
            self.override_value = Some(value.into());
        }
    }

    pub(crate) fn clear_override(&mut self) {
        self.override_value = None;
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum SupportedDeviceFamily {
    IPhone,
    IPad,
    AppleTv,
    AppleWatch,
    Mac,
}

impl SupportedDeviceFamily {
    pub(crate) const ALL: [Self; 5] = [
        Self::IPhone,
        Self::IPad,
        Self::AppleTv,
        Self::AppleWatch,
        Self::Mac,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::IPhone => "iPhone",
            Self::IPad => "iPad",
            Self::AppleTv => "Apple TV",
            Self::AppleWatch => "Apple Watch",
            Self::Mac => "Mac",
        }
    }

    pub(crate) fn from_device_family_id(id: u64) -> Option<Self> {
        match id {
            1 => Some(Self::IPhone),
            2 => Some(Self::IPad),
            3 => Some(Self::AppleTv),
            4 => Some(Self::AppleWatch),
            6 => Some(Self::Mac),
            _ => None,
        }
    }

    pub(crate) fn parse_list(value: &str) -> Vec<Self> {
        let mut families = value
            .split(',')
            .filter_map(|value| Self::from_label(value.trim()))
            .collect::<Vec<_>>();
        normalize_supported_families(&mut families);
        families
    }

    pub(crate) fn display_list(families: &[Self]) -> SharedString {
        if families.is_empty() {
            return "Unknown".into();
        }

        families
            .iter()
            .map(|family| family.label())
            .collect::<Vec<_>>()
            .join(", ")
            .into()
    }

    fn from_label(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "iphone" | "ipod" | "ios" => Some(Self::IPhone),
            "ipad" | "ipados" => Some(Self::IPad),
            "apple tv" | "appletv" | "tvos" => Some(Self::AppleTv),
            "apple watch" | "applewatch" | "watch" | "watchos" => Some(Self::AppleWatch),
            "mac" | "macos" => Some(Self::Mac),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SupportedDevicesValue {
    pub(crate) default: Vec<SupportedDeviceFamily>,
    pub(crate) override_value: Option<Vec<SupportedDeviceFamily>>,
}

impl SupportedDevicesValue {
    pub(crate) fn new(default: Vec<SupportedDeviceFamily>) -> Self {
        Self {
            default,
            override_value: None,
        }
    }

    pub(crate) fn value(&self) -> &[SupportedDeviceFamily] {
        self.override_value.as_deref().unwrap_or(&self.default)
    }

    pub(crate) fn display_value(&self) -> SharedString {
        SupportedDeviceFamily::display_list(self.value())
    }

    pub(crate) fn is_overridden(&self) -> bool {
        self.override_value.is_some()
    }

    pub(crate) fn set_override(&mut self, mut value: Vec<SupportedDeviceFamily>) {
        normalize_supported_families(&mut value);
        if value == self.default {
            self.override_value = None;
        } else {
            self.override_value = Some(value);
        }
    }

    pub(crate) fn clear_override(&mut self) {
        self.override_value = None;
    }
}

fn normalize_supported_families(families: &mut Vec<SupportedDeviceFamily>) {
    families.sort();
    families.dedup();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppMetadataField {
    Name,
    BundleId,
    Version,
    Build,
    Executable,
    MinimumOs,
    SupportedDevices,
}

impl AppMetadataField {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::BundleId => "Bundle ID",
            Self::Version => "Version",
            Self::Build => "Build",
            Self::Executable => "Executable",
            Self::MinimumOs => "Minimum OS",
            Self::SupportedDevices => "Supported devices",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AppMetadata {
    pub(crate) name: AppFieldValue,
    pub(crate) bundle_id: AppFieldValue,
    pub(crate) version: AppFieldValue,
    pub(crate) build: AppFieldValue,
    pub(crate) executable: AppFieldValue,
    pub(crate) minimum_os: AppFieldValue,
    pub(crate) supported_devices: SupportedDevicesValue,
}

impl AppMetadata {
    pub(crate) fn sample(
        name: impl Into<SharedString>,
        bundle_id: impl Into<SharedString>,
        version: impl Into<SharedString>,
        build: impl Into<SharedString>,
        executable: impl Into<SharedString>,
        minimum_os: impl Into<SharedString>,
        supported_devices: Vec<SupportedDeviceFamily>,
    ) -> Self {
        Self {
            name: AppFieldValue::new(name),
            bundle_id: AppFieldValue::new(bundle_id),
            version: AppFieldValue::new(version),
            build: AppFieldValue::new(build),
            executable: AppFieldValue::new(executable),
            minimum_os: AppFieldValue::new(minimum_os),
            supported_devices: SupportedDevicesValue::new(supported_devices),
        }
    }

    pub(crate) fn field(&self, field: AppMetadataField) -> &AppFieldValue {
        match field {
            AppMetadataField::Name => &self.name,
            AppMetadataField::BundleId => &self.bundle_id,
            AppMetadataField::Version => &self.version,
            AppMetadataField::Build => &self.build,
            AppMetadataField::Executable => &self.executable,
            AppMetadataField::MinimumOs => &self.minimum_os,
            AppMetadataField::SupportedDevices => {
                panic!("supported devices are not represented as a text metadata field")
            }
        }
    }

    pub(crate) fn field_mut(&mut self, field: AppMetadataField) -> &mut AppFieldValue {
        match field {
            AppMetadataField::Name => &mut self.name,
            AppMetadataField::BundleId => &mut self.bundle_id,
            AppMetadataField::Version => &mut self.version,
            AppMetadataField::Build => &mut self.build,
            AppMetadataField::Executable => &mut self.executable,
            AppMetadataField::MinimumOs => &mut self.minimum_os,
            AppMetadataField::SupportedDevices => {
                panic!("supported devices are not represented as a text metadata field")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AppEntitlement {
    pub(crate) key: SharedString,
    pub(crate) value: EntitlementValue,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EntitlementValue {
    String(SharedString),
    Boolean(bool),
    Integer(i64),
    Number(f64),
    Array(Vec<EntitlementValue>),
    Dictionary(Vec<(SharedString, EntitlementValue)>),
    Data(Vec<u8>),
    Date(SharedString),
    Uid(u64),
    Unknown(SharedString),
}

impl EntitlementValue {
    pub(crate) const EDITABLE_TYPE_LABELS: [&'static str; 3] = ["String", "Boolean", "Array"];

    pub(crate) fn type_label(&self) -> &'static str {
        match self {
            Self::String(_) => "String",
            Self::Boolean(_) => "Boolean",
            Self::Integer(_) => "Integer",
            Self::Number(_) => "Number",
            Self::Array(_) => "Array",
            Self::Dictionary(_) => "Dictionary",
            Self::Data(_) => "Data",
            Self::Date(_) => "Date",
            Self::Uid(_) => "UID",
            Self::Unknown(_) => "Value",
        }
    }

    pub(crate) fn display_text(&self) -> SharedString {
        self.edit_text().into()
    }

    pub(crate) fn edit_text(&self) -> String {
        match self {
            Self::String(value) => value.to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::Array(values) => values
                .iter()
                .map(Self::compact_text)
                .collect::<Vec<_>>()
                .join(", "),
            Self::Dictionary(values) => format!("{} keys", values.len()),
            Self::Data(value) => format!("{} bytes", value.len()),
            Self::Date(value) => value.to_string(),
            Self::Uid(value) => value.to_string(),
            Self::Unknown(value) => value.to_string(),
        }
    }

    pub(crate) fn with_type_label(&self, label: &str) -> Self {
        Self::from_type_and_text(label, &self.edit_text())
    }

    pub(crate) fn with_edit_text(&self, text: &str) -> Self {
        Self::from_type_and_text(self.type_label(), text)
    }

    pub(crate) fn string_array(values: Vec<String>) -> Self {
        Self::Array(
            values
                .into_iter()
                .map(|value| Self::String(value.into()))
                .collect(),
        )
    }

    pub(crate) fn array_edit_values(&self) -> Vec<String> {
        match self {
            Self::Array(values) => values.iter().map(Self::edit_text).collect(),
            _ => vec![self.edit_text()],
        }
    }

    pub(crate) fn from_type_and_text(label: &str, text: &str) -> Self {
        match label {
            "String" => Self::String(text.to_string().into()),
            "Boolean" => Self::Boolean(matches!(
                text.trim().to_ascii_lowercase().as_str(),
                "true" | "yes" | "1"
            )),
            "Integer" => Self::Integer(text.trim().parse().unwrap_or_default()),
            "Number" => Self::Number(text.trim().parse().unwrap_or_default()),
            "Array" => Self::Array(
                text.split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| Self::String(value.to_string().into()))
                    .collect(),
            ),
            "Dictionary" => Self::Dictionary(Vec::new()),
            "Data" => Self::Data(Vec::new()),
            "Date" => Self::Date(text.to_string().into()),
            "UID" => Self::Uid(text.trim().parse().unwrap_or_default()),
            _ => Self::Unknown(text.to_string().into()),
        }
    }

    fn compact_text(&self) -> String {
        match self {
            Self::Array(values) => format!("{} items", values.len()),
            Self::Dictionary(values) => format!("{} keys", values.len()),
            Self::Data(value) => format!("{} bytes", value.len()),
            _ => self.edit_text(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AppOption {
    pub(crate) metadata: AppMetadata,
    pub(crate) path: SharedString,
    pub(crate) icon_path: Option<SharedString>,
    pub(crate) icon_override_path: Option<SharedString>,
    pub(crate) entitlements: Vec<AppEntitlement>,
    pub(crate) entitlement_overrides: Option<Vec<AppEntitlement>>,
    pub(crate) patches: Vec<PatchOption>,
}

impl AppOption {
    pub(crate) fn name(&self) -> &SharedString {
        self.metadata.name.value()
    }

    pub(crate) fn bundle_id(&self) -> &SharedString {
        self.metadata.bundle_id.value()
    }

    pub(crate) fn version(&self) -> &SharedString {
        self.metadata.version.value()
    }

    pub(crate) fn build(&self) -> &SharedString {
        self.metadata.build.value()
    }

    pub(crate) fn field(&self, field: AppMetadataField) -> &AppFieldValue {
        self.metadata.field(field)
    }

    pub(crate) fn field_display_value(&self, field: AppMetadataField) -> SharedString {
        match field {
            AppMetadataField::SupportedDevices => self.metadata.supported_devices.display_value(),
            field => self.field(field).value().clone(),
        }
    }

    pub(crate) fn field_is_overridden(&self, field: AppMetadataField) -> bool {
        match field {
            AppMetadataField::SupportedDevices => self.metadata.supported_devices.is_overridden(),
            field => self.field(field).is_overridden(),
        }
    }

    pub(crate) fn set_field_override(&mut self, field: AppMetadataField, value: String) {
        match field {
            AppMetadataField::SupportedDevices => self
                .metadata
                .supported_devices
                .set_override(SupportedDeviceFamily::parse_list(&value)),
            field => self.metadata.field_mut(field).set_override(value),
        }
    }

    pub(crate) fn set_supported_devices_override(&mut self, value: Vec<SupportedDeviceFamily>) {
        self.metadata.supported_devices.set_override(value);
    }

    pub(crate) fn clear_field_override(&mut self, field: AppMetadataField) {
        match field {
            AppMetadataField::SupportedDevices => self.metadata.supported_devices.clear_override(),
            field => self.metadata.field_mut(field).clear_override(),
        }
    }

    pub(crate) fn supported_devices(&self) -> &[SupportedDeviceFamily] {
        self.metadata.supported_devices.value()
    }

    pub(crate) fn displayed_icon_path(&self) -> Option<&SharedString> {
        self.icon_override_path.as_ref().or(self.icon_path.as_ref())
    }

    pub(crate) fn effective_entitlements(&self, team_id: &SharedString) -> Vec<AppEntitlement> {
        if let Some(overrides) = self.entitlement_overrides.as_ref() {
            return overrides.clone();
        }

        self.default_effective_entitlements(team_id)
    }

    pub(crate) fn default_effective_entitlements(
        &self,
        team_id: &SharedString,
    ) -> Vec<AppEntitlement> {
        self.entitlements
            .iter()
            .map(|entitlement| team_adjusted_entitlement(entitlement, self.bundle_id(), team_id))
            .collect()
    }

    pub(crate) fn entitlements_are_overridden(&self) -> bool {
        self.entitlement_overrides.is_some()
    }
}

fn team_adjusted_entitlement(
    entitlement: &AppEntitlement,
    bundle_id: &SharedString,
    team_id: &SharedString,
) -> AppEntitlement {
    let mut entitlement = entitlement.clone();
    match entitlement.key.as_ref() {
        "application-identifier" => {
            entitlement.value = EntitlementValue::String(team_prefixed_identifier(
                &entitlement.value.edit_text(),
                bundle_id,
                team_id,
            ));
        }
        "com.apple.developer.team-identifier" => {
            entitlement.value = EntitlementValue::String(team_id.clone());
        }
        "keychain-access-groups" => {
            entitlement.value =
                team_prefixed_keychain_groups(&entitlement.value, bundle_id, team_id);
        }
        _ => {}
    }
    entitlement
}

fn team_prefixed_keychain_groups(
    value: &EntitlementValue,
    bundle_id: &SharedString,
    team_id: &SharedString,
) -> EntitlementValue {
    match value {
        EntitlementValue::Array(values) => EntitlementValue::Array(
            values
                .iter()
                .map(|value| match value {
                    EntitlementValue::String(value) => EntitlementValue::String(
                        team_prefixed_identifier(value.as_ref(), bundle_id, team_id),
                    ),
                    value => value.clone(),
                })
                .collect(),
        ),
        value => EntitlementValue::Array(
            value
                .edit_text()
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    EntitlementValue::String(team_prefixed_identifier(value, bundle_id, team_id))
                })
                .collect(),
        ),
    }
}

fn team_prefixed_identifier(
    value: &str,
    bundle_id: &SharedString,
    team_id: &SharedString,
) -> SharedString {
    let suffix = value
        .split_once('.')
        .map(|(_, suffix)| suffix)
        .filter(|suffix| !suffix.is_empty())
        .unwrap_or(bundle_id.as_ref());
    format!("{team_id}.{suffix}").into()
}

#[derive(Clone, Debug)]
pub(crate) struct DeviceOption {
    pub(crate) name: SharedString,
    pub(crate) model: SharedString,
    pub(crate) os: SharedString,
    pub(crate) udid: SharedString,
    pub(crate) connection: SharedString,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AdiBackendKind {
    SystemAdid,
    WindowsCoreAdi,
    AndroidCoreAdi,
}

#[derive(Clone, Debug)]
pub(crate) struct AdiBackendDetail {
    pub(crate) label: String,
    pub(crate) value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AdiBackendAvailability {
    Ready,
    NeedsSetup,
    Unavailable,
}

impl AdiBackendAvailability {
    pub(crate) fn label(self) -> String {
        match self {
            Self::Ready => "Ready",
            Self::NeedsSetup => "Needs setup",
            Self::Unavailable => "Unavailable",
        }
        .to_string()
    }

    pub(crate) fn is_ready(self) -> bool {
        self == Self::Ready
    }

    pub(crate) fn is_available(self) -> bool {
        self != Self::Unavailable
    }
}

#[derive(Clone, Debug)]
pub(crate) enum AdiProvisioningState {
    Unknown,
    NotAvailable,
    Provisioned,
    NotProvisioned,
    Error(String),
}

impl AdiProvisioningState {
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Unknown => "Unknown",
            Self::NotAvailable => "Not available",
            Self::Provisioned => "Provisioned",
            Self::NotProvisioned => "Not provisioned",
            Self::Error(_) => "Failed",
        }
        .to_string()
    }

    pub(crate) fn detail(&self) -> Option<String> {
        match self {
            Self::Error(error) => Some(error.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AdiRepairAction {
    InstallCoreAdi,
    LocateLibrary,
}

impl AdiRepairAction {
    pub(crate) fn label(self) -> String {
        match self {
            Self::InstallCoreAdi => "Install CoreADI",
            Self::LocateLibrary => "Locate Library",
        }
        .to_string()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AdiBackendOption {
    pub(crate) kind: AdiBackendKind,
    pub(crate) name: String,
    pub(crate) detail: String,
    pub(crate) availability: AdiBackendAvailability,
    pub(crate) details: Vec<AdiBackendDetail>,
    pub(crate) provisioning_state: AdiProvisioningState,
    pub(crate) editable_identity: bool,
    pub(crate) repair_action: Option<AdiRepairAction>,
}

#[derive(Clone, Debug)]
pub(crate) struct MachineIdentity {
    pub(crate) machine_name: SharedString,
    pub(crate) os_name: SharedString,
    pub(crate) os_version: SharedString,
    pub(crate) machine_id: SharedString,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SideloadPhase {
    Signing,
    Installing,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum SideloadOperation {
    Idle,
    Running { phase: SideloadPhase, progress: f32 },
    Finished,
    Failed { message: SharedString },
}

impl SideloadOperation {
    pub(crate) fn is_busy(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    pub(crate) fn progress(&self) -> f32 {
        match self {
            Self::Idle => 0.,
            Self::Running { progress, .. } => progress.clamp(0., 1.),
            Self::Finished => 1.,
            Self::Failed { .. } => 0.,
        }
    }

    pub(crate) fn top_label(&self) -> &'static str {
        match self {
            Self::Idle => "Draft",
            Self::Running { phase, .. } => phase.label(),
            Self::Finished => "Done",
            Self::Failed { .. } => "Failed",
        }
    }

    pub(crate) fn status_color(&self) -> u32 {
        match self {
            Self::Idle => 0x53666d,
            Self::Running { .. } => 0x173f45,
            Self::Finished => 0x1d6b45,
            Self::Failed { .. } => 0x9a302b,
        }
    }

    pub(crate) fn button_label(&self) -> &'static str {
        match self {
            Self::Idle | Self::Finished | Self::Failed { .. } => "Sideload",
            Self::Running { phase, .. } => phase.button_label(),
        }
    }

    pub(crate) fn status_text(
        &self,
        app: Option<&AppOption>,
        device: Option<&DeviceOption>,
    ) -> SharedString {
        match self {
            Self::Idle => {
                if app.is_none() {
                    "Choose an IPA".into()
                } else if device.is_some() {
                    "Ready to install".into()
                } else {
                    "Connect a device".into()
                }
            }
            Self::Running { phase, .. } => match phase {
                SideloadPhase::Signing => {
                    let app_name = app.map(|app| app.name().as_ref()).unwrap_or("app");
                    format!("Signing {app_name}").into()
                }
                SideloadPhase::Installing => {
                    let device_name = device
                        .map(|device| device.name.as_ref())
                        .unwrap_or("device");
                    format!("Installing to {device_name}").into()
                }
            },
            Self::Finished => {
                let device_name = device
                    .map(|device| device.name.as_ref())
                    .unwrap_or("device");
                format!("Installed on {device_name}").into()
            }
            Self::Failed { message } => message.clone(),
        }
    }
}

impl SideloadPhase {
    fn label(self) -> &'static str {
        match self {
            Self::Signing => "Signing",
            Self::Installing => "Installing",
        }
    }

    pub(crate) fn button_label(self) -> &'static str {
        match self {
            Self::Signing => "Signing",
            Self::Installing => "Installing",
        }
    }
}
