use crate::models::{
    AdiBackendAvailability, AdiBackendDetail, AdiBackendKind, AdiBackendOption,
    AdiProvisioningState, AdiRepairAction, MachineIdentity,
};
use crate::paths::app_data_dir;
use adi::core_adi::CoreADIADIProxy;
#[cfg(target_os = "windows")]
use adi::core_adi::{CoreADIParameters, CoreADIProxy};
use adi::ADIProxy;
use async_zip::tokio::read::fs::ZipFileReader;
use futures_lite::io::AsyncReadExt;
use grandslam::bundle_information::BundleInformation;
use grandslam::device::Device;
use grandslam::http_session::AnisetteHTTPSession;
#[cfg(target_os = "windows")]
use ouroboros::self_referencing;
use std::ffi::CString;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const ANDROID_COREADI_APK_URL: &str =
    "https://apps.mzstatic.com/content/android-apple-music-apk/applemusic.apk";
const ANDROID_COREADI_APK_FILE: &str = "applemusic.apk";
const ANDROID_COREADI_LIBRARY_FILE: &str = "libCoreADI.so";
const GRANDSLAM_DSID: i64 = -2;
const XCODE_BUNDLE_INFORMATION: BundleInformation<'static> = BundleInformation {
    bundle_name: "Xcode",
    bundle_identifier: "com.apple.dt.Xcode",
    bundle_version: "23792",
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct AndroidCoreAdiInstallProgress {
    pub(crate) downloaded_bytes: u64,
    pub(crate) total_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum AndroidCoreAdiInstallEvent {
    Downloading(AndroidCoreAdiInstallProgress),
    Installing,
}

pub(crate) fn available_adi_backends(android_adi_identifier: &str) -> Vec<AdiBackendOption> {
    let mut backends: Vec<_> = vec![
        system_adi_backend(),
        windows_coreadi_backend(),
        android_coreadi_backend(android_adi_identifier),
    ]
    .into_iter()
    .filter(|backend| backend.availability.is_available())
    .collect();

    for backend in &mut backends {
        backend.provisioning_state = if backend.availability.is_ready() {
            adi_provisioning_state(backend.kind, android_adi_identifier)
        } else {
            AdiProvisioningState::NotAvailable
        };
    }

    backends
}

pub(crate) fn default_adi_backend(backends: &[AdiBackendOption]) -> usize {
    backends
        .iter()
        .position(|backend| backend.availability.is_ready())
        .unwrap_or(0)
}

#[allow(dead_code)]
pub(crate) fn selected_adi_proxy(
    kind: AdiBackendKind,
    android_adi_identifier: &str,
) -> Result<Box<dyn ADIProxy>, String> {
    match kind {
        AdiBackendKind::SystemAdid => system_adi_proxy(),
        AdiBackendKind::WindowsCoreAdi => windows_coreadi_proxy(),
        AdiBackendKind::AndroidCoreAdi => android_coreadi_proxy(android_adi_identifier),
    }
}

#[allow(dead_code)]
pub(crate) fn erase_adi_provisioning(
    kind: AdiBackendKind,
    android_adi_identifier: &str,
) -> Result<(), String> {
    selected_adi_proxy(kind, android_adi_identifier)?
        .erase_provisioning(GRANDSLAM_DSID)
        .map_err(|error| format!("Failed to erase ADI provisioning: {error}"))
}

pub(crate) fn provision_adi(
    kind: AdiBackendKind,
    machine_identity: &MachineIdentity,
    android_adi_identifier: &str,
) -> Result<(), String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to start ADI provisioning runtime: {error}"))?
        .block_on(provision_adi_async(
            kind,
            machine_identity,
            android_adi_identifier,
        ))
}

async fn provision_adi_async(
    kind: AdiBackendKind,
    machine_identity: &MachineIdentity,
    android_adi_identifier: &str,
) -> Result<(), String> {
    let proxy = selected_adi_proxy(kind, android_adi_identifier)?;
    let http_session =
        grandslam::http_session(grandslam_device(machine_identity), XCODE_BUNDLE_INFORMATION)
            .await
            .map_err(|error| format!("Failed to create ADI provisioning session: {error}"))?;
    let anisette_session = AnisetteHTTPSession::new(http_session, proxy.as_ref());

    grandslam::provision(&anisette_session)
        .await
        .map_err(|error| format!("Failed to provision ADI: {error}"))
}

fn grandslam_device(machine_identity: &MachineIdentity) -> Device {
    Device {
        device_model: machine_identity.machine_name.to_string(),
        operating_system_information: format!(
            "{};{}",
            machine_identity.os_name, machine_identity.os_version
        ),
        device_uuid: machine_identity.machine_id.to_string(),
    }
}

fn adi_provisioning_state(
    kind: AdiBackendKind,
    android_adi_identifier: &str,
) -> AdiProvisioningState {
    match selected_adi_proxy(kind, android_adi_identifier) {
        Ok(proxy) => match proxy.is_machine_provisioned(GRANDSLAM_DSID) {
            Ok(true) => AdiProvisioningState::Provisioned,
            Ok(false) => AdiProvisioningState::NotProvisioned,
            Err(error) => AdiProvisioningState::Error(error.to_string().into()),
        },
        Err(error) => AdiProvisioningState::Error(error.into()),
    }
}

#[cfg(target_os = "macos")]
fn system_adi_proxy() -> Result<Box<dyn ADIProxy>, String> {
    Ok(Box::new(adid_proxy::ADIdProxy::connect()))
}

#[cfg(not(target_os = "macos"))]
fn system_adi_proxy() -> Result<Box<dyn ADIProxy>, String> {
    Err("System ADI is only available on macOS.".to_string())
}

#[cfg(target_os = "windows")]
#[self_referencing]
struct WindowsCoreADIProxy {
    library: dlopen2::symbor::Library,

    #[borrows(library)]
    #[covariant]
    proxy: library_coreadi::LibraryCoreADIProxy<'this>,
}

#[cfg(target_os = "windows")]
impl WindowsCoreADIProxy {
    fn open(path: PathBuf) -> Result<Self, String> {
        let library = dlopen2::symbor::Library::open(path.as_os_str())
            .map_err(|error| format!("Failed to load CoreADI.dll: {error}"))?;
        let proxy = WindowsCoreADIProxyTryBuilder {
            library,
            proxy_builder: |library| {
                library_coreadi::LibraryCoreADIProxy::new(library)
                    .map_err(|error| format!("CoreADI entry point could not be loaded: {error}"))
            },
        }
        .try_build()?;

        proxy
            .with_proxy(|proxy| {
                CoreADIADIProxy::initialize(proxy)
                    .map_err(|error| format!("CoreADI initialization failed: {error}"))
            })
            .map(|()| proxy)
    }
}

#[cfg(target_os = "windows")]
impl CoreADIProxy for WindowsCoreADIProxy {
    unsafe fn dispatch(&self, function_code: u32, parameters: *const CoreADIParameters) -> i32 {
        self.with_proxy(|proxy| unsafe { proxy.dispatch(function_code, parameters) })
    }
}

#[cfg(target_os = "windows")]
fn windows_coreadi_proxy() -> Result<Box<dyn ADIProxy>, String> {
    let library_path = find_windows_coreadi_library().ok_or_else(|| {
        "CoreADI.dll was not found in the usual iTunes or iCloud folders.".to_string()
    })?;
    Ok(Box::new(WindowsCoreADIProxy::open(library_path)?))
}

#[cfg(not(target_os = "windows"))]
fn windows_coreadi_proxy() -> Result<Box<dyn ADIProxy>, String> {
    Err("iTunes or iCloud CoreADI is only available on Windows.".to_string())
}

fn android_coreadi_proxy(android_adi_identifier: &str) -> Result<Box<dyn ADIProxy>, String> {
    let library_path = android_coreadi_library_path()
        .ok_or_else(|| "The application data folder is not available.".to_string())?;
    let library_data = fs::read(&library_path).map_err(|error| {
        format!(
            "Failed to read Apple Music CoreADI at {}: {error}",
            library_path.display()
        )
    })?;

    let proxy = android_coreadi::AndroidCoreADIProxy::load_library(library_data)
        .map_err(|error| format!("Failed to load Apple Music CoreADI: {error}"))?;
    CoreADIADIProxy::initialize(&proxy)
        .map_err(|error| format!("Apple Music CoreADI initialization failed: {error}"))?;

    let provisioning_path = android_coreadi_provisioning_path()
        .ok_or_else(|| "The application data folder is not available.".to_string())?;
    fs::create_dir_all(&provisioning_path).map_err(|error| {
        format!(
            "Failed to create CoreADI provisioning folder at {}: {error}",
            provisioning_path.display()
        )
    })?;
    let provisioning_path = c_string_from_path(&provisioning_path)?;
    CoreADIADIProxy::set_provisioning_path(&proxy, &provisioning_path)
        .map_err(|error| format!("Failed to configure CoreADI provisioning path: {error}"))?;

    if android_adi_identifier.len() != 16 {
        return Err(format!(
            "Apple Music CoreADI ADI identifier must be 16 characters long, got {}.",
            android_adi_identifier.len()
        ));
    }
    CoreADIADIProxy::set_android_id(&proxy, android_adi_identifier)
        .map_err(|error| format!("Failed to configure CoreADI ADI identifier: {error}"))?;

    Ok(Box::new(proxy))
}

fn system_adi_backend() -> AdiBackendOption {
    #[cfg(target_os = "macos")]
    let availability = AdiBackendAvailability::Ready;

    #[cfg(not(target_os = "macos"))]
    let availability = AdiBackendAvailability::Unavailable;

    AdiBackendOption {
        kind: AdiBackendKind::SystemAdid,
        name: "System ADI".into(),
        detail: "Use the platform ADI provider.".into(),
        availability,
        details: vec![AdiBackendDetail {
            label: "Identity storage".into(),
            value: "System managed".into(),
        }],
        provisioning_state: AdiProvisioningState::Unknown,
        editable_identity: false,
        repair_action: None,
    }
}

fn windows_coreadi_backend() -> AdiBackendOption {
    let library_path = find_windows_coreadi_library();
    let availability = if cfg!(target_os = "windows") {
        if library_path.is_some() {
            AdiBackendAvailability::Ready
        } else {
            AdiBackendAvailability::NeedsSetup
        }
    } else {
        AdiBackendAvailability::Unavailable
    };

    AdiBackendOption {
        kind: AdiBackendKind::WindowsCoreAdi,
        name: "iTunes / iCloud CoreADI".into(),
        detail: "Load Apple's Windows ADI library.".into(),
        availability,
        details: vec![
            AdiBackendDetail {
                label: "Library path".into(),
                value: library_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Not found".to_string())
                    .into(),
            },
            AdiBackendDetail {
                label: "Identity storage".into(),
                value: "Apple managed".into(),
            },
        ],
        provisioning_state: AdiProvisioningState::Unknown,
        editable_identity: false,
        repair_action: (availability == AdiBackendAvailability::NeedsSetup)
            .then_some(AdiRepairAction::LocateLibrary),
    }
}

fn android_coreadi_backend(android_adi_identifier: &str) -> AdiBackendOption {
    let Some(abi) = android_coreadi_abi() else {
        return AdiBackendOption {
            kind: AdiBackendKind::AndroidCoreAdi,
            name: "Apple Music CoreADI".into(),
            detail: "Use Android's portable CoreADI library.".into(),
            availability: AdiBackendAvailability::Unavailable,
            details: vec![AdiBackendDetail {
                label: "Architecture".into(),
                value: "Unsupported".into(),
            }],
            provisioning_state: AdiProvisioningState::NotAvailable,
            editable_identity: false,
            repair_action: None,
        };
    };

    let library_path = android_coreadi_library_path();
    let availability = if library_path.as_ref().is_some_and(|path| path.exists()) {
        AdiBackendAvailability::Ready
    } else {
        AdiBackendAvailability::NeedsSetup
    };
    let library_path_label = library_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Not available".to_string());
    let provisioning_path_label = android_coreadi_provisioning_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Not available".to_string());

    AdiBackendOption {
        kind: AdiBackendKind::AndroidCoreAdi,
        name: "Apple Music CoreADI".into(),
        detail: "Use Android's portable CoreADI library.".into(),
        availability,
        details: vec![
            AdiBackendDetail {
                label: "Library path".into(),
                value: library_path_label.into(),
            },
            AdiBackendDetail {
                label: "APK ABI".into(),
                value: abi.into(),
            },
            AdiBackendDetail {
                label: "ADI identifier".into(),
                value: android_adi_identifier.to_string().into(),
            },
            AdiBackendDetail {
                label: "Provisioning path".into(),
                value: provisioning_path_label.into(),
            },
        ],
        provisioning_state: if availability.is_ready() {
            AdiProvisioningState::Unknown
        } else {
            AdiProvisioningState::NotAvailable
        },
        editable_identity: true,
        repair_action: (availability == AdiBackendAvailability::NeedsSetup)
            .then_some(AdiRepairAction::InstallCoreAdi),
    }
}

pub(crate) fn android_coreadi_library_path() -> Option<PathBuf> {
    app_data_dir()
        .zip(android_coreadi_abi())
        .map(|(path, abi)| path.join(abi).join(ANDROID_COREADI_LIBRARY_FILE))
}

fn android_coreadi_apk_path() -> Option<PathBuf> {
    app_data_dir().map(|path| path.join(ANDROID_COREADI_APK_FILE))
}

fn android_coreadi_provisioning_path() -> Option<PathBuf> {
    app_data_dir().map(|path| path.join("coreadi"))
}

pub(crate) fn download_android_coreadi_apk(
    mut progress: impl FnMut(AndroidCoreAdiInstallProgress),
) -> Result<PathBuf, String> {
    let _ = android_coreadi_abi().ok_or_else(|| {
        format!(
            "Apple Music CoreADI is not available for this CPU architecture ({})",
            std::env::consts::ARCH
        )
    })?;
    let destination = android_coreadi_apk_path()
        .ok_or_else(|| "The application data folder is not available.".to_string())?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create Apple Music APK folder at {}: {error}",
                parent.display()
            )
        })?;
    }

    let mut response = reqwest::blocking::Client::new()
        .get(ANDROID_COREADI_APK_URL)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("Failed to download Apple Music APK: {error}"))?;
    let total_bytes = response.content_length();
    let mut file = File::create(&destination).map_err(|error| {
        format!(
            "Failed to create Apple Music APK at {}: {error}",
            destination.display()
        )
    })?;
    let mut downloaded_bytes = 0;
    let mut buffer = [0; 64 * 1024];
    progress(AndroidCoreAdiInstallProgress {
        downloaded_bytes,
        total_bytes,
    });

    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| format!("Failed to download Apple Music APK: {error}"))?;
        if read == 0 {
            break;
        }

        file.write_all(&buffer[..read])
            .map_err(|error| format!("Failed to save Apple Music APK: {error}"))?;
        downloaded_bytes += read as u64;
        progress(AndroidCoreAdiInstallProgress {
            downloaded_bytes,
            total_bytes,
        });
    }

    file.flush().map_err(|error| {
        format!(
            "Failed to flush Apple Music APK at {}: {error}",
            destination.display()
        )
    })?;

    if downloaded_bytes == 0 {
        return Err("Downloaded Apple Music APK is empty.".to_string());
    }

    Ok(destination)
}

pub(crate) fn download_and_install_android_coreadi(
    mut progress: impl FnMut(AndroidCoreAdiInstallEvent),
) -> Result<PathBuf, String> {
    let apk_path = download_android_coreadi_apk(|download| {
        progress(AndroidCoreAdiInstallEvent::Downloading(download));
    })?;
    progress(AndroidCoreAdiInstallEvent::Installing);
    install_android_coreadi_from_apk(&apk_path)
}

#[allow(dead_code)]
pub(crate) fn install_android_coreadi_from_apk(apk_path: &Path) -> Result<PathBuf, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to start CoreADI APK reader runtime: {error}"))?
        .block_on(install_android_coreadi_from_apk_async(apk_path))
}

async fn install_android_coreadi_from_apk_async(apk_path: &Path) -> Result<PathBuf, String> {
    let entry_name = android_coreadi_apk_entry().ok_or_else(|| {
        format!(
            "Apple Music CoreADI is not available for this CPU architecture ({})",
            std::env::consts::ARCH
        )
    })?;
    let reader = ZipFileReader::new(apk_path).await.map_err(|error| {
        format!(
            "Failed to read APK archive at {}: {error}",
            apk_path.display()
        )
    })?;
    let entry_index = reader
        .file()
        .entries()
        .iter()
        .position(|entry| {
            entry
                .filename()
                .as_str()
                .is_ok_and(|filename| filename == entry_name)
        })
        .ok_or_else(|| format!("APK does not contain {entry_name}."))?;

    let mut entry_reader = reader
        .reader_without_entry(entry_index)
        .await
        .map_err(|error| format!("Failed to read CoreADI from APK: {error}"))?;

    let destination = android_coreadi_library_path()
        .ok_or_else(|| "The application data folder is not available.".to_string())?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create CoreADI folder at {}: {error}",
                parent.display()
            )
        })?;
    }
    let mut destination_file = File::create(&destination).map_err(|error| {
        format!(
            "Failed to create CoreADI destination at {}: {error}",
            destination.display()
        )
    })?;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = entry_reader
            .read(&mut buffer)
            .await
            .map_err(|error| format!("Failed to extract CoreADI from APK: {error}"))?;
        if read == 0 {
            break;
        }
        destination_file
            .write_all(&buffer[..read])
            .map_err(|error| {
                format!(
                    "Failed to store CoreADI at {}: {error}",
                    destination.display()
                )
            })?;
    }
    destination_file.flush().map_err(|error| {
        format!(
            "Failed to flush CoreADI at {}: {error}",
            destination.display()
        )
    })?;

    Ok(destination)
}

#[cfg(target_arch = "aarch64")]
fn android_coreadi_abi() -> Option<&'static str> {
    Some("arm64-v8a")
}

#[cfg(target_arch = "arm")]
fn android_coreadi_abi() -> Option<&'static str> {
    Some("armeabi-v7a")
}

#[cfg(target_arch = "x86")]
fn android_coreadi_abi() -> Option<&'static str> {
    Some("x86")
}

#[cfg(target_arch = "x86_64")]
fn android_coreadi_abi() -> Option<&'static str> {
    Some("x86_64")
}

#[cfg(not(any(
    target_arch = "aarch64",
    target_arch = "arm",
    target_arch = "x86",
    target_arch = "x86_64"
)))]
fn android_coreadi_abi() -> Option<&'static str> {
    None
}

fn android_coreadi_apk_entry() -> Option<String> {
    android_coreadi_abi().map(|abi| format!("lib/{abi}/{ANDROID_COREADI_LIBRARY_FILE}"))
}

fn c_string_from_path(path: &Path) -> Result<CString, String> {
    CString::new(path.to_string_lossy().into_owned())
        .map_err(|_| format!("Path contains a null byte: {}", path.display()))
}

fn find_windows_coreadi_library() -> Option<PathBuf> {
    if !cfg!(target_os = "windows") {
        return None;
    }

    windows_coreadi_candidates()
        .into_iter()
        .find(|path| path.exists())
}

fn windows_coreadi_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for env_var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = std::env::var_os(env_var) {
            let root = PathBuf::from(root);
            candidates.push(
                root.join("Common Files")
                    .join("Apple")
                    .join("Apple Application Support")
                    .join("CoreADI.dll"),
            );
            candidates.push(
                root.join("Apple")
                    .join("Apple Application Support")
                    .join("CoreADI.dll"),
            );
        }
    }
    if let Some(root) = std::env::var_os("CommonProgramFiles") {
        candidates.push(
            PathBuf::from(root)
                .join("Apple")
                .join("Apple Application Support")
                .join("CoreADI.dll"),
        );
    }
    candidates
}
