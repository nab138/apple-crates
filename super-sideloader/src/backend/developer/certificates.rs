use crate::backend::paths::app_data_dir;
use crate::backend::{BackendError, BackendResult};
use sha1::Digest as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Clone, Debug)]
pub(crate) struct GeneratedCertificateSigningRequest {
    pub(crate) machine_id: String,
    pub(crate) machine_name: String,
    pub(crate) csr_content: String,
    pub(crate) public_key_fingerprint: String,
    pub(crate) private_key_pem: Vec<u8>,
}

pub(crate) struct AppManagedSigningMaterial {
    pub(crate) private_key_pem: Vec<u8>,
    pub(crate) certificate_der: Vec<u8>,
}

pub(crate) fn generate_development_certificate_signing_request(
) -> BackendResult<GeneratedCertificateSigningRequest> {
    let machine_id = Uuid::new_v4().to_string().to_uppercase();
    let machine_name = "Super Sideloader".to_string();
    let temp_dir = certificate_temp_dir()?;
    fs::create_dir_all(&temp_dir).map_err(|source| BackendError::Io {
        action: "Create certificate work folder",
        path: temp_dir.clone(),
        source,
    })?;
    let file_id = Uuid::new_v4().to_string();
    let key_path = temp_dir.join(format!("{file_id}.key.pem"));
    let csr_path = temp_dir.join(format!("{file_id}.csr.pem"));
    let subject = format!("/CN={machine_name}");

    let mut generate = Command::new("openssl");
    generate
        .args(["req", "-new", "-newkey", "rsa:2048", "-nodes"])
        .arg("-keyout")
        .arg(&key_path)
        .arg("-out")
        .arg(&csr_path)
        .arg("-subj")
        .arg(subject)
        .arg("-batch");
    run_command(generate, "generate a development certificate CSR")?;

    let private_key_pem = fs::read(&key_path).map_err(|source| BackendError::Io {
        action: "Read generated private key",
        path: key_path.clone(),
        source,
    })?;
    let csr_content = fs::read_to_string(&csr_path).map_err(|source| BackendError::Io {
        action: "Read generated CSR",
        path: csr_path.clone(),
        source,
    })?;
    let public_key_der = public_key_der_from_private_key(&key_path)?;
    let public_key_fingerprint = certificate_fingerprint(&public_key_der);

    let _ = fs::remove_file(&key_path);
    let _ = fs::remove_file(&csr_path);

    Ok(GeneratedCertificateSigningRequest {
        machine_id,
        machine_name,
        csr_content,
        public_key_fingerprint,
        private_key_pem,
    })
}

pub(crate) fn import_app_managed_private_key(
    certificate_id: &str,
    expected_public_key_fingerprint: &str,
    private_key_path: &Path,
) -> BackendResult<()> {
    let private_key_pem = fs::read(private_key_path).map_err(|source| BackendError::Io {
        action: "Read private key",
        path: private_key_path.to_path_buf(),
        source,
    })?;
    let public_key_der = public_key_der_from_private_key(private_key_path)?;
    let public_key_fingerprint = certificate_fingerprint(&public_key_der);
    if !public_key_fingerprint.eq_ignore_ascii_case(expected_public_key_fingerprint) {
        return Err(BackendError::Keychain(format!(
            "The selected PEM private key does not match certificate {certificate_id}."
        )));
    }

    save_app_managed_private_key(&public_key_fingerprint, &private_key_pem)
}

pub(crate) fn certificate_fingerprint(contents: &[u8]) -> String {
    sha1::Sha1::digest(contents)
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect()
}

pub(crate) fn certificate_public_key_fingerprint(certificate_der: &[u8]) -> Option<String> {
    certificate_public_key_der(certificate_der)
        .ok()
        .map(|public_key_der| certificate_fingerprint(&public_key_der))
}

pub(crate) fn app_managed_private_key_fingerprints() -> Vec<String> {
    let Ok(keys_dir) = certificate_keys_dir() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(keys_dir) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .path()
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_ascii_uppercase)
        })
        .filter(|fingerprint| {
            fingerprint.len() == 40 && fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .collect()
}

pub(crate) fn save_app_managed_certificate(
    fingerprint: &str,
    certificate_der: &[u8],
) -> BackendResult<()> {
    let certificates_dir = signing_certificates_dir()?;
    fs::create_dir_all(&certificates_dir).map_err(|source| BackendError::Io {
        action: "Create signing certificate folder",
        path: certificates_dir.clone(),
        source,
    })?;
    let certificate_path = certificates_dir.join(format!("{}.der", safe_fingerprint(fingerprint)?));
    fs::write(&certificate_path, certificate_der).map_err(|source| BackendError::Io {
        action: "Save signing certificate",
        path: certificate_path,
        source,
    })
}

pub(crate) fn load_app_managed_signing_material(
    certificate_fingerprint: &str,
    public_key_fingerprint: &str,
) -> BackendResult<AppManagedSigningMaterial> {
    let certificate_path = signing_certificates_dir()?.join(format!(
        "{}.der",
        safe_fingerprint(certificate_fingerprint)?
    ));
    let certificate_der = read_signing_resource(
        &certificate_path,
        "Read signing certificate",
        "The selected certificate data is not cached. Refresh Developer Settings, then try signing again.",
    )?;

    let private_key_path =
        certificate_keys_dir()?.join(format!("{}.pem", safe_fingerprint(public_key_fingerprint)?));
    let private_key_pem = read_signing_resource(
        &private_key_path,
        "Read certificate private key",
        "The selected certificate has no Super Sideloader managed private key. Create a certificate or import its matching PEM key in Developer Settings.",
    )?;

    Ok(AppManagedSigningMaterial {
        private_key_pem,
        certificate_der,
    })
}

fn public_key_der_from_private_key(key_path: &Path) -> BackendResult<Vec<u8>> {
    let mut command = Command::new("openssl");
    command
        .args(["pkey", "-in"])
        .arg(key_path)
        .args(["-pubout", "-outform", "DER"]);
    command_output(command, "extract the private key public key")
}

fn certificate_public_key_der(certificate_der: &[u8]) -> BackendResult<Vec<u8>> {
    let temp_dir = certificate_temp_dir()?;
    fs::create_dir_all(&temp_dir).map_err(|source| BackendError::Io {
        action: "Create certificate work folder",
        path: temp_dir.clone(),
        source,
    })?;
    let file_id = Uuid::new_v4().to_string();
    let certificate_path = temp_dir.join(format!("{file_id}.cer"));
    let public_key_path = temp_dir.join(format!("{file_id}.pub.pem"));
    fs::write(&certificate_path, certificate_der).map_err(|source| BackendError::Io {
        action: "Stage certificate",
        path: certificate_path.clone(),
        source,
    })?;

    let mut extract_public_key = Command::new("openssl");
    extract_public_key
        .args(["x509", "-inform", "DER", "-in"])
        .arg(&certificate_path)
        .args(["-pubkey", "-noout", "-out"])
        .arg(&public_key_path);
    let result =
        run_command(extract_public_key, "extract the certificate public key").and_then(|_| {
            let mut convert_public_key = Command::new("openssl");
            convert_public_key
                .args(["pkey", "-pubin", "-in"])
                .arg(&public_key_path)
                .args(["-outform", "DER"]);
            command_output(convert_public_key, "encode the certificate public key")
        });

    let _ = fs::remove_file(&certificate_path);
    let _ = fs::remove_file(&public_key_path);
    result
}

pub(crate) fn save_app_managed_private_key(
    fingerprint: &str,
    private_key_pem: &[u8],
) -> BackendResult<()> {
    let keys_dir = certificate_keys_dir()?;
    fs::create_dir_all(&keys_dir).map_err(|source| BackendError::Io {
        action: "Create certificate key folder",
        path: keys_dir.clone(),
        source,
    })?;
    let key_path = keys_dir.join(format!("{}.pem", safe_fingerprint(fingerprint)?));
    fs::write(&key_path, private_key_pem).map_err(|source| BackendError::Io {
        action: "Save certificate private key",
        path: key_path.clone(),
        source,
    })?;
    secure_private_key_file(&key_path)
}

#[cfg(unix)]
fn secure_private_key_file(path: &Path) -> BackendResult<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|source| {
        BackendError::Io {
            action: "Secure certificate private key",
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn secure_private_key_file(_: &Path) -> BackendResult<()> {
    Ok(())
}

fn read_signing_resource(
    path: &Path,
    action: &'static str,
    missing_message: &str,
) -> BackendResult<Vec<u8>> {
    fs::read(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            BackendError::Message(missing_message.to_string())
        } else {
            BackendError::Io {
                action,
                path: path.to_path_buf(),
                source,
            }
        }
    })
}

fn safe_fingerprint(fingerprint: &str) -> BackendResult<String> {
    let fingerprint = fingerprint.trim().to_ascii_uppercase();
    if fingerprint.len() == 40 && fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(fingerprint)
    } else {
        Err(BackendError::Cache(
            "Signing certificate cache contains an invalid fingerprint.".to_string(),
        ))
    }
}

fn certificate_keys_dir() -> BackendResult<PathBuf> {
    app_data_dir()
        .map(|path| path.join("certificates").join("keys"))
        .ok_or_else(|| {
            BackendError::Unsupported("The application data folder is not available.".to_string())
        })
}

fn signing_certificates_dir() -> BackendResult<PathBuf> {
    app_data_dir()
        .map(|path| path.join("certificates").join("certificates"))
        .ok_or_else(|| {
            BackendError::Unsupported("The application data folder is not available.".to_string())
        })
}

fn certificate_temp_dir() -> BackendResult<PathBuf> {
    app_data_dir()
        .map(|path| path.join("certificates").join("tmp"))
        .ok_or_else(|| {
            BackendError::Unsupported("The application data folder is not available.".to_string())
        })
}

fn run_command(mut command: Command, action: &'static str) -> BackendResult<()> {
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|source| BackendError::Command { action, source })?;
    if output.status.success() {
        return Ok(());
    }
    Err(command_error(action, &output.stderr))
}

fn command_output(mut command: Command, action: &'static str) -> BackendResult<Vec<u8>> {
    let output = command
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|source| BackendError::Command { action, source })?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(command_error(action, &output.stderr))
}

fn command_error(action: &'static str, stderr: &[u8]) -> BackendError {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    let detail = if stderr.is_empty() {
        "Process exited with a non-zero status.".to_string()
    } else {
        stderr
    };
    BackendError::CommandFailed { action, detail }
}
