#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Device {
    pub(crate) name: String,
    pub(crate) model: String,
    pub(crate) os: String,
    pub(crate) udid: String,
    pub(crate) connection: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DeviceWatchEvent {
    Changed,
    Failed(String),
}
