use crate::backend::ipa::SignedAppBundle;
use crate::backend::{runtime as backend_runtime, BackendError, BackendResult};
use crate::domain::{Device, DeviceInstallProgress, DeviceWatchEvent};
use futures::{channel::mpsc::UnboundedSender, StreamExt};
use idevice::{
    afc::{opcode::AfcFopenMode, AfcClient},
    provider::UsbmuxdProvider,
    services::installation_proxy::InstallationProxyClient,
    services::lockdown::LockdownClient,
    usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdDevice},
    IdeviceService,
};
use plist::Value;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use walkdir::WalkDir;

const APP_UPLOAD_BUFFER_SIZE: usize = 1024 * 1024;

pub(crate) async fn discover_devices() -> BackendResult<Vec<Device>> {
    backend_runtime::run_send("device discovery", discover_devices_async()).await?
}

pub(crate) async fn watch_device_changes(
    sender: UnboundedSender<DeviceWatchEvent>,
) -> BackendResult<()> {
    backend_runtime::run("device watcher", move || watch_device_changes_async(sender)).await?
}

pub(crate) async fn install_app(
    udid: String,
    signed_app: SignedAppBundle,
    progress: impl FnMut(DeviceInstallProgress) + Send + 'static,
) -> BackendResult<()> {
    backend_runtime::run_send(
        "app installation",
        install_app_async(udid, signed_app, progress),
    )
    .await?
}

async fn install_app_async(
    udid: String,
    signed_app: SignedAppBundle,
    progress: impl FnMut(DeviceInstallProgress) + Send + 'static,
) -> BackendResult<()> {
    let progress = Arc::new(Mutex::new(progress));
    report_install_progress(&progress, DeviceInstallProgress::Connecting);

    let provider = provider_for_udid(&udid).await?;
    let app_path = signed_app.path().to_path_buf();
    let upload_plan = app_upload_plan(&app_path)?;

    let mut afc = AfcClient::connect(&provider).await.map_err(|error| {
        BackendError::DeviceInstall(format!("Unable to open AFC on {udid}: {error}"))
    })?;
    if afc.get_file_info("PublicStaging").await.is_err() {
        afc.mk_dir("PublicStaging").await.map_err(|error| {
            BackendError::DeviceInstall(format!(
                "Unable to create the device staging directory: {error}"
            ))
        })?;
    }

    let app_folder_name = app_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| name.ends_with(".app"))
        .ok_or_else(|| {
            BackendError::DeviceInstall(format!(
                "Signed app path {} does not end in an .app folder.",
                app_path.display()
            ))
        })?;
    let remote_path = format!("PublicStaging/{app_folder_name}");
    if afc.get_file_info(&remote_path).await.is_ok() {
        afc.remove_all(&remote_path).await.map_err(|error| {
            BackendError::DeviceInstall(format!(
                "Unable to replace the existing staged app folder: {error}"
            ))
        })?;
    }
    afc.mk_dir(&remote_path).await.map_err(|error| {
        BackendError::DeviceInstall(format!("Unable to create the staged app folder: {error}"))
    })?;
    if let Err(error) = upload_app_bundle(&mut afc, &remote_path, &upload_plan, &progress).await {
        remove_staged_app(&mut afc, &remote_path).await;
        return Err(error);
    }

    let mut options = plist::Dictionary::new();
    options.insert(
        "PackageType".to_string(),
        Value::String("Developer".to_string()),
    );
    let mut installer = match InstallationProxyClient::connect(&provider).await {
        Ok(installer) => installer,
        Err(error) => {
            remove_staged_app(&mut afc, &remote_path).await;
            return Err(BackendError::DeviceInstall(format!(
                "Unable to open InstallationProxy on {udid}: {error}"
            )));
        }
    };
    let install_progress = Arc::clone(&progress);
    let result = installer
        .install_with_callback(
            remote_path.clone(),
            Some(Value::Dictionary(options)),
            move |(percent, progress)| async move {
                report_install_progress(&progress, DeviceInstallProgress::Installing { percent });
            },
            install_progress,
        )
        .await;

    report_install_progress(&progress, DeviceInstallProgress::Finalizing);
    remove_staged_app(&mut afc, &remote_path).await;
    drop(signed_app);

    result.map_err(|error| {
        BackendError::DeviceInstall(format!("Unable to install the app on {udid}: {error}"))
    })
}

struct AppUploadPlan {
    entries: Vec<AppUploadEntry>,
    total_bytes: u64,
    total_files: usize,
}

struct AppUploadEntry {
    local_path: PathBuf,
    relative_path: String,
    is_directory: bool,
}

fn app_upload_plan(app_path: &Path) -> BackendResult<AppUploadPlan> {
    let mut entries = Vec::new();
    let mut total_bytes = 0_u64;
    let mut total_files = 0_usize;

    for entry in WalkDir::new(app_path).min_depth(1).sort_by_file_name() {
        let entry = entry.map_err(|error| {
            BackendError::DeviceInstall(format!("Unable to inspect signed app files: {error}"))
        })?;
        let relative = entry.path().strip_prefix(app_path).map_err(|error| {
            BackendError::DeviceInstall(format!("Unable to stage signed app path: {error}"))
        })?;
        let relative_path = device_relative_path(relative)?;
        let file_type = entry.file_type();
        if file_type.is_dir() {
            entries.push(AppUploadEntry {
                local_path: entry.into_path(),
                relative_path,
                is_directory: true,
            });
        } else if file_type.is_file() {
            let metadata = entry.metadata().map_err(|error| {
                BackendError::DeviceInstall(format!(
                    "Unable to inspect {}: {error}",
                    entry.path().display()
                ))
            })?;
            total_bytes = total_bytes.saturating_add(metadata.len());
            total_files += 1;
            entries.push(AppUploadEntry {
                local_path: entry.into_path(),
                relative_path,
                is_directory: false,
            });
        } else {
            return Err(BackendError::DeviceInstall(format!(
                "Signed app contains unsupported file {}.",
                entry.path().display()
            )));
        }
    }

    Ok(AppUploadPlan {
        entries,
        total_bytes,
        total_files,
    })
}

fn device_relative_path(path: &Path) -> BackendResult<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        let Component::Normal(component) = component else {
            return Err(BackendError::DeviceInstall(format!(
                "Signed app contains unsafe path {}.",
                path.display()
            )));
        };
        parts.push(component.to_string_lossy().into_owned());
    }
    if parts.is_empty() {
        return Err(BackendError::DeviceInstall(
            "Signed app contains an empty relative path.".to_string(),
        ));
    }
    Ok(parts.join("/"))
}

async fn upload_app_bundle<P>(
    afc: &mut AfcClient,
    remote_root: &str,
    plan: &AppUploadPlan,
    progress: &Arc<Mutex<P>>,
) -> BackendResult<()>
where
    P: FnMut(DeviceInstallProgress),
{
    let mut transferred_bytes = 0_u64;
    let mut completed_files = 0_usize;
    report_install_progress(
        progress,
        DeviceInstallProgress::Uploading {
            transferred_bytes,
            total_bytes: plan.total_bytes,
            completed_files,
            total_files: plan.total_files,
        },
    );

    for entry in &plan.entries {
        let remote_path = format!("{remote_root}/{}", entry.relative_path);
        if entry.is_directory {
            afc.mk_dir(&remote_path).await.map_err(|error| {
                BackendError::DeviceInstall(format!(
                    "Unable to create staged folder {remote_path}: {error}"
                ))
            })?;
            continue;
        }

        let mut source = tokio::fs::File::open(&entry.local_path)
            .await
            .map_err(|error| {
                BackendError::DeviceInstall(format!(
                    "Unable to open signed app file {}: {error}",
                    entry.local_path.display()
                ))
            })?;
        let mut destination =
            afc.open(&remote_path, AfcFopenMode::WrOnly)
                .await
                .map_err(|error| {
                    BackendError::DeviceInstall(format!(
                        "Unable to stage app file {remote_path}: {error}"
                    ))
                })?;
        let mut buffer = vec![0; APP_UPLOAD_BUFFER_SIZE];
        let upload_result = async {
            loop {
                let read = source.read(&mut buffer).await.map_err(|error| {
                    BackendError::DeviceInstall(format!(
                        "Unable to read signed app file {}: {error}",
                        entry.local_path.display()
                    ))
                })?;
                if read == 0 {
                    return Ok(());
                }
                destination
                    .write_all(&buffer[..read])
                    .await
                    .map_err(|error| {
                        BackendError::DeviceInstall(format!(
                            "Unable to upload app file {remote_path}: {error}"
                        ))
                    })?;
                transferred_bytes = transferred_bytes.saturating_add(read as u64);
                report_install_progress(
                    progress,
                    DeviceInstallProgress::Uploading {
                        transferred_bytes,
                        total_bytes: plan.total_bytes,
                        completed_files,
                        total_files: plan.total_files,
                    },
                );
            }
        }
        .await;
        let close_result = destination.close().await.map_err(|error| {
            BackendError::DeviceInstall(format!("Unable to finish app file {remote_path}: {error}"))
        });
        upload_result.and(close_result)?;
        completed_files += 1;
        report_install_progress(
            progress,
            DeviceInstallProgress::Uploading {
                transferred_bytes,
                total_bytes: plan.total_bytes,
                completed_files,
                total_files: plan.total_files,
            },
        );
    }

    Ok(())
}

async fn remove_staged_app(afc: &mut AfcClient, remote_path: &str) {
    if let Err(error) = afc.remove_all(remote_path).await {
        log::warn!("Unable to remove staged app {remote_path}: {error}");
    }
}

async fn provider_for_udid(udid: &str) -> BackendResult<UsbmuxdProvider> {
    let addr = UsbmuxdAddr::from_env_var().map_err(|error| {
        BackendError::DeviceInstall(format!("Invalid usbmuxd address: {error}"))
    })?;
    let mut usbmuxd = addr.connect(100).await.map_err(|error| {
        BackendError::DeviceInstall(format!("Unable to connect to usbmuxd: {error}"))
    })?;
    let device = usbmuxd
        .get_devices()
        .await
        .map_err(|error| {
            BackendError::DeviceInstall(format!("Unable to list connected devices: {error}"))
        })?
        .into_iter()
        .find(|device| device.udid == udid)
        .ok_or_else(|| {
            BackendError::DeviceInstall(format!(
                "Device {udid} is no longer connected. Reconnect it and try again."
            ))
        })?;

    Ok(UsbmuxdProvider {
        addr,
        tag: 101,
        udid: device.udid,
        device_id: device.device_id,
        label: "super-sideloader.install".to_string(),
    })
}

fn report_install_progress<P>(progress: &Arc<Mutex<P>>, event: DeviceInstallProgress)
where
    P: FnMut(DeviceInstallProgress),
{
    if let Ok(mut progress) = progress.lock() {
        progress(event);
    }
}

async fn watch_device_changes_async(
    sender: UnboundedSender<DeviceWatchEvent>,
) -> BackendResult<()> {
    let addr = UsbmuxdAddr::from_env_var().map_err(|error| {
        BackendError::DeviceDiscovery(format!("Invalid usbmuxd address: {error}"))
    })?;
    let mut usbmuxd = addr.connect(1).await.map_err(|error| {
        BackendError::DeviceDiscovery(format!("Unable to connect to usbmuxd: {error}"))
    })?;
    let mut events = usbmuxd.listen().await.map_err(|error| {
        BackendError::DeviceDiscovery(format!("Unable to listen for device changes: {error}"))
    })?;

    while let Some(event) = events.next().await {
        match event {
            Ok(_) => {
                if sender.unbounded_send(DeviceWatchEvent::Changed).is_err() {
                    return Ok(());
                }
            }
            Err(error) => {
                let message = format!("Unable to read device change: {error}");
                let _ = sender.unbounded_send(DeviceWatchEvent::Failed(message.clone()));
                return Err(BackendError::DeviceDiscovery(message));
            }
        }
    }

    let message = "Device change listener ended".to_string();
    let _ = sender.unbounded_send(DeviceWatchEvent::Failed(message.clone()));
    Err(BackendError::DeviceDiscovery(message))
}

async fn discover_devices_async() -> BackendResult<Vec<Device>> {
    let addr = UsbmuxdAddr::from_env_var().map_err(|error| {
        BackendError::DeviceDiscovery(format!("Invalid usbmuxd address: {error}"))
    })?;
    let mut usbmuxd = addr.connect(1).await.map_err(|error| {
        BackendError::DeviceDiscovery(format!("Unable to connect to usbmuxd: {error}"))
    })?;
    let devices = usbmuxd.get_devices().await.map_err(|error| {
        BackendError::DeviceDiscovery(format!("Unable to list devices: {error}"))
    })?;

    let mut discovered = Vec::with_capacity(devices.len());
    for (index, device) in devices.into_iter().enumerate() {
        discovered.push(describe_device(addr.clone(), index as u32 + 2, device).await);
    }

    Ok(discovered)
}

async fn describe_device(addr: UsbmuxdAddr, tag: u32, device: UsbmuxdDevice) -> Device {
    let udid = device.udid.clone();
    let connection = connection_label(&device.connection_type);
    let provider = UsbmuxdProvider {
        addr,
        tag,
        udid: device.udid,
        device_id: device.device_id,
        label: "super-sideloader".to_string(),
    };

    let mut name = "Connected Device".to_string();
    let mut model = "Unknown model".to_string();
    let mut os = "Unknown OS".to_string();

    if let Ok(mut lockdown) = LockdownClient::connect(&provider).await {
        name = lockdown_string(&mut lockdown, "DeviceName")
            .await
            .filter(|value| !value.is_empty())
            .unwrap_or(name);
        model = lockdown_string(&mut lockdown, "ProductType")
            .await
            .filter(|value| !value.is_empty())
            .unwrap_or(model);

        let version = lockdown_string(&mut lockdown, "ProductVersion").await;
        let class = lockdown_string(&mut lockdown, "DeviceClass").await;
        if let Some(version) = version.filter(|value| !value.is_empty()) {
            os = format!("{} {version}", os_name(class.as_deref(), &model));
        }
    }

    Device {
        name,
        model,
        os,
        udid,
        connection,
    }
}

async fn lockdown_string(lockdown: &mut LockdownClient, key: &str) -> Option<String> {
    match lockdown.get_value(Some(key), None).await.ok()? {
        Value::String(value) => Some(value),
        _ => None,
    }
}

fn connection_label(connection: &Connection) -> String {
    match connection {
        Connection::Usb => "USB".to_string(),
        Connection::Network(_) => "Wi-Fi".to_string(),
        Connection::Unknown(label) => label.clone(),
    }
}

fn os_name(device_class: Option<&str>, model: &str) -> &'static str {
    match device_class {
        Some("iPad") => "iPadOS",
        Some("AppleTV") => "tvOS",
        Some("Watch") => "watchOS",
        Some("iPhone") | Some("iPod") => "iOS",
        _ if model.starts_with("iPad") => "iPadOS",
        _ if model.starts_with("AppleTV") => "tvOS",
        _ if model.starts_with("Watch") => "watchOS",
        _ => "iOS",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn app_upload_plan_counts_files_bytes_and_device_paths() {
        let temp = tempfile::tempdir().unwrap();
        let app = temp.path().join("Example.app");
        let plug_in = app.join("PlugIns").join("Widget.appex");
        fs::create_dir_all(&plug_in).unwrap();
        fs::write(app.join("Info.plist"), b"plist").unwrap();
        fs::write(plug_in.join("Widget"), b"executable").unwrap();

        let plan = app_upload_plan(&app).unwrap();

        assert_eq!(plan.total_files, 2);
        assert_eq!(plan.total_bytes, 15);
        assert!(plan.entries.iter().any(|entry| {
            entry.relative_path == "PlugIns/Widget.appex/Widget" && !entry.is_directory
        }));
        assert_eq!(
            device_relative_path(Path::new("Frameworks/Example.framework/Example")).unwrap(),
            "Frameworks/Example.framework/Example"
        );
    }
}
