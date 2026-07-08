use crate::models::DeviceOption;
use futures::{channel::mpsc::UnboundedSender, StreamExt};
use idevice::{
    provider::UsbmuxdProvider,
    services::lockdown::LockdownClient,
    usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdDevice},
    IdeviceService,
};
use plist::Value;

pub(crate) fn discover_devices() -> Result<Vec<DeviceOption>, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("Unable to start device runtime: {error}"))?;

    runtime.block_on(discover_devices_async())
}

pub(crate) fn watch_device_changes(
    sender: UnboundedSender<Result<(), String>>,
) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("Unable to start device runtime: {error}"))?;

    runtime.block_on(watch_device_changes_async(sender))
}

async fn watch_device_changes_async(
    sender: UnboundedSender<Result<(), String>>,
) -> Result<(), String> {
    let addr =
        UsbmuxdAddr::from_env_var().map_err(|error| format!("Invalid usbmuxd address: {error}"))?;
    let mut usbmuxd = addr
        .connect(1)
        .await
        .map_err(|error| format!("Unable to connect to usbmuxd: {error}"))?;
    let mut events = usbmuxd
        .listen()
        .await
        .map_err(|error| format!("Unable to listen for device changes: {error}"))?;

    while let Some(event) = events.next().await {
        match event {
            Ok(_) => {
                if sender.unbounded_send(Ok(())).is_err() {
                    return Ok(());
                }
            }
            Err(error) => {
                let message = format!("Unable to read device change: {error}");
                let _ = sender.unbounded_send(Err(message.clone()));
                return Err(message);
            }
        }
    }

    let message = "Device change listener ended".to_string();
    let _ = sender.unbounded_send(Err(message.clone()));
    Err(message)
}

async fn discover_devices_async() -> Result<Vec<DeviceOption>, String> {
    let addr =
        UsbmuxdAddr::from_env_var().map_err(|error| format!("Invalid usbmuxd address: {error}"))?;
    let mut usbmuxd = addr
        .connect(1)
        .await
        .map_err(|error| format!("Unable to connect to usbmuxd: {error}"))?;
    let devices = usbmuxd
        .get_devices()
        .await
        .map_err(|error| format!("Unable to list devices: {error}"))?;

    let mut discovered = Vec::with_capacity(devices.len());
    for (index, device) in devices.into_iter().enumerate() {
        discovered.push(describe_device(addr.clone(), index as u32 + 2, device).await);
    }

    Ok(discovered)
}

async fn describe_device(addr: UsbmuxdAddr, tag: u32, device: UsbmuxdDevice) -> DeviceOption {
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

    DeviceOption {
        name: name.into(),
        model: model.into(),
        os: os.into(),
        udid: udid.into(),
        connection: connection.into(),
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
