use crate::models::{
    AccountOption, AdiBackendOption, AppIdOption, AppOption, DeviceOption, MachineIdentity,
    PatchOption, TeamOption,
};
use std::env;

pub(crate) fn sample_accounts() -> Vec<AccountOption> {
    let env_apple_id = env::var("APPLE_ID").unwrap_or_else(|_| "Add Apple ID".to_string());
    let env_detail = if env_apple_id == "Add Apple ID" {
        "No saved credentials".to_string()
    } else {
        "Loaded from APPLE_ID".to_string()
    };

    vec![
        AccountOption {
            label: "Primary Apple ID".into(),
            apple_id: env_apple_id.into(),
            detail: env_detail.into(),
            status: "Available".into(),
            teams: vec![
                TeamOption {
                    name: "Personal Team".into(),
                    identifier: "LOCAL-PERSONAL".into(),
                    role: "Free".into(),
                    app_ids: sample_app_ids("LOCAL-PERSONAL"),
                },
                TeamOption {
                    name: "Developer Program".into(),
                    identifier: "TEAMID1234".into(),
                    role: "Paid".into(),
                    app_ids: sample_app_ids("TEAMID1234"),
                },
            ],
        },
        AccountOption {
            label: "Work Apple ID".into(),
            apple_id: "developer@example.com".into(),
            detail: "Example account placeholder".into(),
            status: "Demo".into(),
            teams: vec![
                TeamOption {
                    name: "Example Apps LLC".into(),
                    identifier: "EXAMPL3APP".into(),
                    role: "Admin".into(),
                    app_ids: sample_app_ids("EXAMPL3APP"),
                },
                TeamOption {
                    name: "Client Signing Team".into(),
                    identifier: "CL13NTTEAM".into(),
                    role: "Member".into(),
                    app_ids: sample_app_ids("CL13NTTEAM"),
                },
            ],
        },
    ]
}

fn sample_app_ids(team_id: &str) -> Vec<AppIdOption> {
    vec![
        AppIdOption {
            name: "Sample App".into(),
            identifier: format!("{team_id}.com.example.sample").into(),
            kind: "Explicit App ID".into(),
        },
        AppIdOption {
            name: "Wildcard Development".into(),
            identifier: format!("{team_id}.*").into(),
            kind: "Wildcard App ID".into(),
        },
    ]
}

pub(crate) fn sample_apps() -> Vec<AppOption> {
    let ipa_path = env::var("IPA_PATH").unwrap_or_else(|_| "/Applications/Sample.ipa".to_string());
    let bundle_id = env::var("BUNDLE_ID").unwrap_or_else(|_| "com.example.sample".to_string());

    vec![
        AppOption {
            name: "Sample IPA".into(),
            bundle_id: bundle_id.into(),
            version: "1.4.2".into(),
            build: "82".into(),
            path: ipa_path.into(),
            patches: vec![
                PatchOption {
                    name: "Preserve Bundle ID".into(),
                    detail: "Do not rewrite CFBundleIdentifier.".into(),
                },
                PatchOption {
                    name: "Enable Documents".into(),
                    detail: "Expose the app documents directory in Files.".into(),
                },
                PatchOption {
                    name: "Strip Watch Extension".into(),
                    detail: "Remove watchOS payloads before signing.".into(),
                },
            ],
        },
        AppOption {
            name: "Debug Build".into(),
            bundle_id: "com.example.debug".into(),
            version: "0.9.0".into(),
            build: "dev-17".into(),
            path: "/tmp/debug-build.ipa".into(),
            patches: vec![
                PatchOption {
                    name: "Inject Get-Task-Allow".into(),
                    detail: "Allow debugger attachment for development.".into(),
                },
                PatchOption {
                    name: "Remove Beta Expiry".into(),
                    detail: "Patch known testflight-style expiry metadata.".into(),
                },
            ],
        },
    ]
}

pub(crate) fn sample_devices() -> Vec<DeviceOption> {
    let udid = env::var("DEVICE_UDID").unwrap_or_else(|_| "00008110-001234DEADBEEF01".to_string());

    vec![
        DeviceOption {
            name: "Connected iPhone".into(),
            model: "iPhone16,2".into(),
            os: "iOS 18.6".into(),
            udid: udid.into(),
            connection: "USB".into(),
        },
        DeviceOption {
            name: "Test iPad".into(),
            model: "iPad14,3".into(),
            os: "iPadOS 18.5".into(),
            udid: "00008103-00A1B2C3D4E5F607".into(),
            connection: "Wi-Fi".into(),
        },
    ]
}

pub(crate) fn sample_adi_backends() -> Vec<AdiBackendOption> {
    vec![
        AdiBackendOption {
            name: "Native ADI".into(),
            detail: "Use the platform ADI provider for this machine.".into(),
            status: "Ready".into(),
            information: "Uses the local machine identity and platform services for ADI requests."
                .into(),
            editable_identity: false,
            repair_action: None,
        },
        AdiBackendOption {
            name: "Provisioned Files".into(),
            detail: "Use a portable ADI identity stored on disk.".into(),
            status: "Needs setup".into(),
            information:
                "Loads ADI provisioning files from the selected profile directory when available."
                    .into(),
            editable_identity: true,
            repair_action: Some("Fix Provisioning".into()),
        },
        AdiBackendOption {
            name: "Remote Proxy".into(),
            detail: "Forward ADI requests to a configured service.".into(),
            status: "Offline".into(),
            information:
                "Connects to an external ADI service and uses its reported machine identity.".into(),
            editable_identity: false,
            repair_action: Some("Reconnect".into()),
        },
    ]
}

pub(crate) fn sample_machine_identity() -> MachineIdentity {
    let machine_name = env::var("HOSTNAME")
        .or_else(|_| env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "Local Machine".to_string());
    let os_name = env::consts::OS.to_string();
    let os_version = env::var("OS_VERSION").unwrap_or_else(|_| "Unknown".to_string());
    let machine_id = env::var("MACHINE_ID")
        .unwrap_or_else(|_| "A8B31C86-359B-4D95-8950-BA5DD8FFC46F".to_string());

    MachineIdentity {
        machine_name: machine_name.into(),
        os_name: os_name.into(),
        os_version: os_version.into(),
        machine_id: machine_id.into(),
    }
}
