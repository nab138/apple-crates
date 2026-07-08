use crate::adi_backend;
use crate::models::{AdiBackendKind, AdiBackendOption, MachineIdentity};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct CoreAdiInstallProgress {
    pub(crate) downloaded_bytes: u64,
    pub(crate) total_bytes: Option<u64>,
}

#[derive(Clone, Debug)]
pub(crate) enum CoreAdiInstallEvent {
    Downloading(CoreAdiInstallProgress),
    Installing,
}

pub(crate) fn available_backends(android_adi_identifier: &str) -> Vec<AdiBackendOption> {
    adi_backend::available_adi_backends(android_adi_identifier)
}

pub(crate) fn default_backend(backends: &[AdiBackendOption]) -> usize {
    adi_backend::default_adi_backend(backends)
}

pub(crate) fn download_and_install_coreadi(
    mut progress: impl FnMut(CoreAdiInstallEvent),
) -> Result<PathBuf, String> {
    adi_backend::download_and_install_android_coreadi(move |event| {
        progress(match event {
            adi_backend::AndroidCoreAdiInstallEvent::Downloading(progress) => {
                CoreAdiInstallEvent::Downloading(CoreAdiInstallProgress {
                    downloaded_bytes: progress.downloaded_bytes,
                    total_bytes: progress.total_bytes,
                })
            }
            adi_backend::AndroidCoreAdiInstallEvent::Installing => CoreAdiInstallEvent::Installing,
        });
    })
}

pub(crate) fn install_coreadi_from_apk(apk_path: &Path) -> Result<PathBuf, String> {
    adi_backend::install_android_coreadi_from_apk(apk_path)
}

pub(crate) fn erase_provisioning(
    kind: AdiBackendKind,
    android_adi_identifier: &str,
) -> Result<(), String> {
    adi_backend::erase_adi_provisioning(kind, android_adi_identifier)
}

pub(crate) fn provision(
    kind: AdiBackendKind,
    machine_identity: &MachineIdentity,
    android_adi_identifier: &str,
) -> Result<(), String> {
    adi_backend::provision_adi(kind, machine_identity, android_adi_identifier)
}
