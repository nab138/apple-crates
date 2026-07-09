use crate::backend::paths::app_data_dir;
use crate::backend::{runtime as backend_runtime, BackendError, BackendResult};
use crate::domain::{
    AppEntitlement, AppMetadata, EntitlementValue, EntitlementsSource, IpaApp, Patch,
    SupportedDeviceFamily,
};
use async_zip::tokio::read::fs::ZipFileReader;
use futures_lite::io::AsyncReadExt;
use plist::{Dictionary, Value};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

pub(crate) async fn read_ipa(path: PathBuf, patches: Vec<Patch>) -> BackendResult<IpaApp> {
    backend_runtime::run_send("IPA reader", read_ipa_async(path, patches)).await?
}

async fn read_ipa_async(path: PathBuf, patches: Vec<Patch>) -> BackendResult<IpaApp> {
    let reader = ZipFileReader::new(&path)
        .await
        .map_err(|error| BackendError::Zip(format!("{}: {error}", path.display())))?;

    let info_index = reader
        .file()
        .entries()
        .iter()
        .position(|entry| entry_name(entry).is_some_and(is_app_info_plist))
        .ok_or_else(|| {
            BackendError::Zip("IPA does not contain Payload/*.app/Info.plist.".to_string())
        })?;
    let info_path = entry_name(&reader.file().entries()[info_index])
        .ok_or_else(|| BackendError::Zip("IPA Info.plist entry name is invalid.".to_string()))?;
    let app_prefix = info_path
        .strip_suffix("Info.plist")
        .ok_or_else(|| BackendError::Zip("IPA Info.plist entry path is invalid.".to_string()))?
        .to_string();

    let info_bytes = read_entry(&reader, info_index, "Info.plist").await?;
    let info = plist::from_bytes::<Dictionary>(&info_bytes)
        .map_err(|error| BackendError::Plist(format!("IPA Info.plist: {error}")))?;

    let bundle_id = required_plist_string(&info, "CFBundleIdentifier")?;
    let executable = required_plist_string(&info, "CFBundleExecutable")?;
    let version = required_plist_string(&info, "CFBundleShortVersionString")?;
    let build = required_plist_string(&info, "CFBundleVersion")?;
    let minimum_os = required_plist_string(&info, "MinimumOSVersion")?;
    let name = plist_string(&info, "CFBundleDisplayName")
        .or_else(|| plist_string(&info, "CFBundleName"))
        .unwrap_or(executable);
    let supported_devices = supported_devices(&info);

    let icon_path = extract_icon(&reader, &app_prefix, &info, bundle_id).await;
    let (entitlements_source, entitlements) = match extract_entitlements(&reader, &app_prefix).await
    {
        Some(entitlements) => (EntitlementsSource::Embedded, entitlements),
        None => (EntitlementsSource::GeneratedFallback, Vec::new()),
    };

    Ok(IpaApp {
        metadata: AppMetadata {
            name: name.to_string(),
            bundle_id: bundle_id.to_string(),
            version: version.to_string(),
            build: build.to_string(),
            executable: executable.to_string(),
            minimum_os: minimum_os.to_string(),
            supported_devices,
        },
        path: path.to_string_lossy().to_string(),
        icon_path: icon_path.map(|path| path.to_string_lossy().to_string()),
        entitlements,
        entitlements_source,
        patches,
    })
}

fn is_app_info_plist(path: &str) -> bool {
    path.starts_with("Payload/")
        && path.ends_with(".app/Info.plist")
        && path["Payload/".len()..].contains(".app/")
}

fn entry_name(entry: &async_zip::ZipEntry) -> Option<&str> {
    entry.filename().as_str().ok()
}

fn required_plist_string<'a>(info: &'a Dictionary, key: &str) -> BackendResult<&'a str> {
    plist_string(info, key)
        .ok_or_else(|| BackendError::Plist(format!("IPA Info.plist is missing string key {key}.")))
}

async fn read_entry(reader: &ZipFileReader, index: usize, label: &str) -> BackendResult<Vec<u8>> {
    let mut entry_reader = reader
        .reader_without_entry(index)
        .await
        .map_err(|error| BackendError::Zip(format!("Failed to read {label} from IPA: {error}")))?;
    let mut data = Vec::new();
    entry_reader.read_to_end(&mut data).await.map_err(|error| {
        BackendError::Zip(format!("Failed to extract {label} from IPA: {error}"))
    })?;
    Ok(data)
}

fn plist_string<'a>(plist: &'a Dictionary, key: &str) -> Option<&'a str> {
    plist.get(key).and_then(Value::as_string)
}

fn supported_devices(info: &Dictionary) -> Vec<SupportedDeviceFamily> {
    let Some(Value::Array(families)) = info.get("UIDeviceFamily") else {
        return Vec::new();
    };

    let mut devices = families
        .iter()
        .filter_map(|value| value.as_unsigned_integer())
        .filter_map(SupportedDeviceFamily::from_device_family_id)
        .collect::<Vec<_>>();
    devices.sort();
    devices.dedup();
    devices
}

async fn extract_icon(
    reader: &ZipFileReader,
    app_prefix: &str,
    info: &Dictionary,
    bundle_id: &str,
) -> Option<PathBuf> {
    let mut candidates = icon_candidates(info);
    candidates.reverse();

    for candidate in candidates {
        if let Some((index, name)) = find_icon_entry(reader, app_prefix, &candidate) {
            if let Ok(bytes) = read_entry(reader, index, name).await {
                if let Some(path) = cache_icon(bundle_id, &bytes) {
                    return Some(path);
                }
            }
        }
    }

    let fallback = reader
        .file()
        .entries()
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let name = entry_name(entry)?;
            let lower = name.to_ascii_lowercase();
            (name.starts_with(app_prefix)
                && lower.ends_with(".png")
                && (lower.contains("appicon") || lower.contains("icon")))
            .then_some((index, name, entry.compressed_size()))
        })
        .max_by_key(|(_, _, size)| *size)
        .map(|(index, name, _)| (index, name));

    let (index, name) = fallback?;
    let bytes = read_entry(reader, index, name).await.ok()?;
    cache_icon(bundle_id, &bytes)
}

fn icon_candidates(info: &Dictionary) -> Vec<String> {
    let mut candidates = Vec::new();
    append_icon_candidates(info.get("CFBundleIconFiles"), &mut candidates);
    append_icon_candidates_from_icons(info.get("CFBundleIcons"), &mut candidates);
    append_icon_candidates_from_icons(info.get("CFBundleIcons~ipad"), &mut candidates);
    candidates
}

fn append_icon_candidates(value: Option<&Value>, candidates: &mut Vec<String>) {
    match value {
        Some(Value::Array(values)) => {
            candidates.extend(
                values
                    .iter()
                    .filter_map(Value::as_string)
                    .map(str::to_string),
            );
        }
        Some(Value::String(value)) => candidates.push(value.clone()),
        _ => {}
    }
}

fn append_icon_candidates_from_icons(value: Option<&Value>, candidates: &mut Vec<String>) {
    let Some(Value::Dictionary(icons)) = value else {
        return;
    };
    let Some(Value::Dictionary(primary_icon)) = icons.get("CFBundlePrimaryIcon") else {
        return;
    };
    append_icon_candidates(primary_icon.get("CFBundleIconFiles"), candidates);
}

fn find_icon_entry<'a>(
    reader: &'a ZipFileReader,
    app_prefix: &str,
    candidate: &str,
) -> Option<(usize, &'a str)> {
    let candidate = candidate.trim_start_matches('/');
    let forms = if candidate.ends_with(".png") {
        vec![candidate.to_string()]
    } else {
        vec![
            format!("{candidate}@3x.png"),
            format!("{candidate}@2x.png"),
            format!("{candidate}.png"),
            candidate.to_string(),
        ]
    };

    reader
        .file()
        .entries()
        .iter()
        .enumerate()
        .find_map(|(index, entry)| {
            let name = entry_name(entry)?;
            let relative = name.strip_prefix(app_prefix)?;
            forms
                .iter()
                .any(|form| relative == form || relative.ends_with(&format!("/{form}")))
                .then_some((index, name))
        })
}

fn cache_icon(bundle_id: &str, bytes: &[u8]) -> Option<PathBuf> {
    let mut path = app_data_dir()?.join("icons");
    fs::create_dir_all(&path).ok()?;
    path.push(format!("{}.png", cache_safe_name(bundle_id)));
    fs::write(&path, bytes).ok()?;
    Some(path)
}

fn cache_safe_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

async fn extract_entitlements(
    reader: &ZipFileReader,
    app_prefix: &str,
) -> Option<Vec<AppEntitlement>> {
    let entry_index = reader.file().entries().iter().position(|entry| {
        entry_name(entry) == Some(&format!("{app_prefix}embedded.mobileprovision"))
    })?;
    let bytes = read_entry(reader, entry_index, "embedded.mobileprovision")
        .await
        .ok()?;
    let plist = embedded_plist(&bytes)?;
    let entitlements = plist.get("Entitlements")?.as_dictionary()?;
    Some(entitlements_from_dictionary(entitlements))
}

fn embedded_plist(bytes: &[u8]) -> Option<Dictionary> {
    let start = find_bytes(bytes, b"<?xml")?;
    let end = find_bytes(&bytes[start..], b"</plist>")? + start + b"</plist>".len();
    plist::from_reader(Cursor::new(&bytes[start..end])).ok()
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn entitlements_from_dictionary(entitlements: &Dictionary) -> Vec<AppEntitlement> {
    entitlements
        .iter()
        .map(|(key, value)| AppEntitlement {
            key: key.clone(),
            value: entitlement_value(value),
        })
        .collect()
}

fn entitlement_value(value: &Value) -> EntitlementValue {
    match value {
        Value::String(value) => EntitlementValue::String(value.clone()),
        Value::Boolean(value) => EntitlementValue::Boolean(*value),
        Value::Integer(value) => value
            .as_signed()
            .map(EntitlementValue::Integer)
            .unwrap_or_else(|| EntitlementValue::String(value.to_string())),
        Value::Real(value) => EntitlementValue::Number(*value),
        Value::Array(values) => {
            EntitlementValue::Array(values.iter().map(entitlement_value).collect())
        }
        Value::Dictionary(values) => EntitlementValue::Dictionary(
            values
                .iter()
                .map(|(key, value)| (key.clone(), entitlement_value(value)))
                .collect(),
        ),
        Value::Data(value) => EntitlementValue::Data(value.clone()),
        Value::Date(value) => EntitlementValue::Date(value.to_xml_format()),
        Value::Uid(value) => EntitlementValue::Uid(value.get()),
        _ => EntitlementValue::Unknown("Value".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_plist_paths_require_payload_app_prefix() {
        assert!(is_app_info_plist("Payload/App.app/Info.plist"));
        assert!(is_app_info_plist("Payload/Nested/App.app/Info.plist"));
        assert!(!is_app_info_plist("App.app/Info.plist"));
        assert!(!is_app_info_plist("Payload/App.app/Other.plist"));
    }

    #[test]
    fn required_plist_string_reports_missing_key() {
        let mut info = Dictionary::new();
        info.insert(
            "CFBundleIdentifier".to_string(),
            Value::String("com.example.app".to_string()),
        );

        assert_eq!(
            required_plist_string(&info, "CFBundleIdentifier").unwrap(),
            "com.example.app"
        );
        assert_eq!(
            required_plist_string(&info, "CFBundleName")
                .unwrap_err()
                .user_message(),
            "Failed to parse plist data: IPA Info.plist is missing string key CFBundleName."
        );
    }

    #[test]
    fn supported_device_family_values_are_normalized() {
        let mut info = Dictionary::new();
        info.insert(
            "UIDeviceFamily".to_string(),
            Value::Array(vec![
                Value::Integer(2.into()),
                Value::Integer(1.into()),
                Value::Integer(2.into()),
                Value::Integer(99.into()),
            ]),
        );

        assert_eq!(
            supported_devices(&info),
            vec![SupportedDeviceFamily::IPhone, SupportedDeviceFamily::IPad]
        );
    }

    #[test]
    fn entitlement_values_are_mapped_without_ui_types() {
        let mut entitlements = Dictionary::new();
        entitlements.insert(
            "application-identifier".to_string(),
            Value::String("TEAM.com.example.app".to_string()),
        );
        entitlements.insert("get-task-allow".to_string(), Value::Boolean(true));

        let mapped = entitlements_from_dictionary(&entitlements);

        assert!(mapped.iter().any(|entitlement| {
            entitlement.key == "application-identifier"
                && entitlement.value == EntitlementValue::String("TEAM.com.example.app".into())
        }));
        assert!(mapped.iter().any(|entitlement| {
            entitlement.key == "get-task-allow"
                && entitlement.value == EntitlementValue::Boolean(true)
        }));
    }
}
