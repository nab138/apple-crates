use crate::backend::adi;
use crate::backend::BackendResult;
use crate::domain::{AdiBackend, AdiBackendKind, MachineIdentity};
use std::path::PathBuf;

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

pub(crate) fn available_backends(android_adi_identifier: &str) -> Vec<AdiBackend> {
    adi::available_adi_backends(android_adi_identifier, false)
}

pub(crate) fn available_backends_with_provisioning(
    android_adi_identifier: &str,
) -> Vec<AdiBackend> {
    adi::available_adi_backends(android_adi_identifier, true)
}

pub(crate) async fn download_and_install_coreadi(
    mut progress: impl FnMut(CoreAdiInstallEvent) + Send + 'static,
) -> BackendResult<PathBuf> {
    adi::download_and_install_android_coreadi(move |event| {
        progress(match event {
            adi::AndroidCoreAdiInstallEvent::Downloading(progress) => {
                CoreAdiInstallEvent::Downloading(CoreAdiInstallProgress {
                    downloaded_bytes: progress.downloaded_bytes,
                    total_bytes: progress.total_bytes,
                })
            }
            adi::AndroidCoreAdiInstallEvent::Installing => CoreAdiInstallEvent::Installing,
        });
    })
    .await
}

pub(crate) async fn install_coreadi_from_apk(apk_path: PathBuf) -> BackendResult<PathBuf> {
    adi::install_android_coreadi_from_apk(apk_path).await
}

pub(crate) fn erase_provisioning(
    kind: AdiBackendKind,
    android_adi_identifier: &str,
) -> BackendResult<()> {
    adi::erase_adi_provisioning(kind, android_adi_identifier)
}

pub(crate) async fn provision(
    kind: AdiBackendKind,
    machine_identity: &MachineIdentity,
    android_adi_identifier: &str,
) -> BackendResult<()> {
    adi::provision_adi(kind, machine_identity, android_adi_identifier).await
}
