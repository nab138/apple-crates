#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PickerId {
    Account,
    Device,
    SideloadAction,
}

#[derive(Clone, Debug)]
pub(crate) struct AppIdCapabilityOption {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) detail: String,
    pub(crate) enabled: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct AppIdOption {
    pub(crate) developer_id: String,
    pub(crate) name: String,
    pub(crate) identifier: String,
    pub(crate) kind: String,
    pub(crate) capabilities: Vec<AppIdCapabilityOption>,
}

#[derive(Clone, Debug)]
pub(crate) struct DevelopmentCertificateOption {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) serial_number: String,
    pub(crate) machine_name: String,
    pub(crate) private_key_available: bool,
    pub(crate) public_key_fingerprint: Option<String>,
}

impl DevelopmentCertificateOption {
    pub(crate) fn private_key_status(&self) -> &'static str {
        if self.private_key_available {
            "Private key available"
        } else {
            "Private key missing"
        }
    }

    pub(crate) fn detail(&self) -> String {
        let machine_name = self.machine_name.trim();
        if machine_name.is_empty() {
            self.private_key_status().to_string()
        } else {
            format!("{machine_name} - {}", self.private_key_status())
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TeamOption {
    pub(crate) name: String,
    pub(crate) identifier: String,
    pub(crate) role: String,
    pub(crate) app_id_available_quantity: Option<u64>,
    pub(crate) app_id_max_quantity: Option<u64>,
    pub(crate) app_ids: Vec<AppIdOption>,
    pub(crate) certificates: Vec<DevelopmentCertificateOption>,
}

#[derive(Clone, Debug)]
pub(crate) struct AccountOption {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) apple_id: String,
    pub(crate) detail: String,
    pub(crate) status: String,
    pub(crate) teams: Vec<TeamOption>,
}

#[derive(Clone, Debug)]
pub(crate) struct PatchOption {
    pub(crate) name: String,
    pub(crate) detail: String,
}

#[derive(Clone, Debug)]
pub(crate) struct AppFieldValue {
    pub(crate) default: String,
    pub(crate) override_value: Option<String>,
}

impl AppFieldValue {
    pub(crate) fn new(default: impl Into<String>) -> Self {
        Self {
            default: default.into(),
            override_value: None,
        }
    }

    pub(crate) fn value(&self) -> &String {
        self.override_value.as_ref().unwrap_or(&self.default)
    }

    pub(crate) fn is_overridden(&self) -> bool {
        self.override_value.is_some()
    }

    pub(crate) fn set_override(&mut self, value: String) {
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

    pub(crate) fn parse_list(value: &str) -> Vec<Self> {
        let mut families = value
            .split(',')
            .filter_map(|value| Self::from_label(value.trim()))
            .collect::<Vec<_>>();
        normalize_supported_families(&mut families);
        families
    }

    pub(crate) fn display_list(families: &[Self]) -> String {
        if families.is_empty() {
            return "Unknown".to_string();
        }

        families
            .iter()
            .map(|family| family.label())
            .collect::<Vec<_>>()
            .join(", ")
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

    pub(crate) fn display_value(&self) -> String {
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
        name: impl Into<String>,
        bundle_id: impl Into<String>,
        version: impl Into<String>,
        build: impl Into<String>,
        executable: impl Into<String>,
        minimum_os: impl Into<String>,
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
    pub(crate) key: String,
    pub(crate) value: EntitlementValue,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EntitlementValue {
    String(String),
    Boolean(bool),
    Integer(i64),
    Number(f64),
    Array(Vec<EntitlementValue>),
    Dictionary(Vec<(String, EntitlementValue)>),
    Data(Vec<u8>),
    Date(String),
    Uid(u64),
    Unknown(String),
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

    pub(crate) fn display_text(&self) -> String {
        self.edit_text()
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
        Self::Array(values.into_iter().map(Self::String).collect())
    }

    pub(crate) fn array_edit_values(&self) -> Vec<String> {
        match self {
            Self::Array(values) => values.iter().map(Self::edit_text).collect(),
            _ => vec![self.edit_text()],
        }
    }

    pub(crate) fn from_type_and_text(label: &str, text: &str) -> Self {
        match label {
            "String" => Self::String(text.to_string()),
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
                    .map(|value| Self::String(value.to_string()))
                    .collect(),
            ),
            "Dictionary" => Self::Dictionary(Vec::new()),
            "Data" => Self::Data(Vec::new()),
            "Date" => Self::Date(text.to_string()),
            "UID" => Self::Uid(text.trim().parse().unwrap_or_default()),
            _ => Self::Unknown(text.to_string()),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EntitlementsSource {
    Embedded,
    GeneratedFallback,
}

#[derive(Clone, Debug)]
pub(crate) struct AppOption {
    pub(crate) metadata: AppMetadata,
    pub(crate) path: String,
    pub(crate) icon_path: Option<String>,
    pub(crate) icon_override_path: Option<String>,
    pub(crate) entitlements: Vec<AppEntitlement>,
    pub(crate) entitlements_source: EntitlementsSource,
    pub(crate) entitlement_overrides: Option<Vec<AppEntitlement>>,
    pub(crate) patches: Vec<PatchOption>,
}

impl AppOption {
    pub(crate) fn name(&self) -> &String {
        self.metadata.name.value()
    }

    pub(crate) fn bundle_id(&self) -> &String {
        self.metadata.bundle_id.value()
    }

    pub(crate) fn version(&self) -> &String {
        self.metadata.version.value()
    }

    pub(crate) fn build(&self) -> &String {
        self.metadata.build.value()
    }

    pub(crate) fn field(&self, field: AppMetadataField) -> &AppFieldValue {
        self.metadata.field(field)
    }

    pub(crate) fn field_display_value(&self, field: AppMetadataField) -> String {
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

    pub(crate) fn displayed_icon_path(&self) -> Option<&String> {
        self.icon_override_path.as_ref().or(self.icon_path.as_ref())
    }

    pub(crate) fn entitlements_are_overridden(&self) -> bool {
        self.entitlement_overrides.is_some()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DeviceOption {
    pub(crate) name: String,
    pub(crate) model: String,
    pub(crate) os: String,
    pub(crate) udid: String,
    pub(crate) connection: String,
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
    pub(crate) machine_name: String,
    pub(crate) os_name: String,
    pub(crate) os_version: String,
    pub(crate) machine_id: String,
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
    Failed { message: String },
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
}
