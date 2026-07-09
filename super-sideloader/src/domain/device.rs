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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DeviceInstallProgress {
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
