use gpui::SharedString;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PickerId {
    Account,
    App,
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
pub(crate) struct AppOption {
    pub(crate) name: SharedString,
    pub(crate) bundle_id: SharedString,
    pub(crate) version: SharedString,
    pub(crate) build: SharedString,
    pub(crate) path: SharedString,
    pub(crate) patches: Vec<PatchOption>,
}

#[derive(Clone, Debug)]
pub(crate) struct DeviceOption {
    pub(crate) name: SharedString,
    pub(crate) model: SharedString,
    pub(crate) os: SharedString,
    pub(crate) udid: SharedString,
    pub(crate) connection: SharedString,
}

#[derive(Clone, Debug)]
pub(crate) struct AdiBackendOption {
    pub(crate) name: SharedString,
    pub(crate) detail: SharedString,
    pub(crate) status: SharedString,
    pub(crate) information: SharedString,
    pub(crate) editable_identity: bool,
    pub(crate) repair_action: Option<SharedString>,
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

    pub(crate) fn status_text(&self, app: &AppOption, device: &DeviceOption) -> SharedString {
        match self {
            Self::Idle => "Ready to install".into(),
            Self::Running { phase, .. } => match phase {
                SideloadPhase::Signing => format!("Signing {}", app.name).into(),
                SideloadPhase::Installing => format!("Installing to {}", device.name).into(),
            },
            Self::Finished => format!("Installed on {}", device.name).into(),
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
