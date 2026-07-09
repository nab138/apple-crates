#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AdiBackendKind {
    SystemAdid,
    WindowsCoreAdi,
    AndroidCoreAdi,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub(crate) fn is_ready(self) -> bool {
        self == Self::Ready
    }

    pub(crate) fn is_available(self) -> bool {
        self != Self::Unavailable
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AdiProvisioningState {
    Unknown,
    NotAvailable,
    Provisioned,
    NotProvisioned,
    Error(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AdiRepairAction {
    InstallCoreAdi,
    LocateLibrary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AdiBackend {
    pub(crate) kind: AdiBackendKind,
    pub(crate) name: String,
    pub(crate) detail: String,
    pub(crate) availability: AdiBackendAvailability,
    pub(crate) details: Vec<AdiBackendDetail>,
    pub(crate) provisioning_state: AdiProvisioningState,
    pub(crate) editable_identity: bool,
    pub(crate) repair_action: Option<AdiRepairAction>,
}
