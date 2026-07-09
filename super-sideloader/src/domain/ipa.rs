#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Patch {
    pub(crate) name: String,
    pub(crate) detail: String,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AppMetadata {
    pub(crate) name: String,
    pub(crate) bundle_id: String,
    pub(crate) version: String,
    pub(crate) build: String,
    pub(crate) executable: String,
    pub(crate) minimum_os: String,
    pub(crate) supported_devices: Vec<SupportedDeviceFamily>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EntitlementsSource {
    Embedded,
    GeneratedFallback,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct IpaApp {
    pub(crate) metadata: AppMetadata,
    pub(crate) path: String,
    pub(crate) icon_path: Option<String>,
    pub(crate) entitlements: Vec<AppEntitlement>,
    pub(crate) entitlements_source: EntitlementsSource,
    pub(crate) patches: Vec<Patch>,
}
