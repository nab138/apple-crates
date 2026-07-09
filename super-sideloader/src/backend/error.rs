use thiserror::Error;

pub(crate) type BackendResult<T> = Result<T, BackendError>;

#[derive(Debug, Error)]
pub(crate) enum BackendError {
    #[error("{action} at {} failed: {source}", path.display())]
    Io {
        action: &'static str,
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{action} failed: {source}")]
    Command {
        action: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("{action} failed: {detail}")]
    CommandFailed {
        action: &'static str,
        detail: String,
    },
    #[error("Failed to parse plist data: {0}")]
    Plist(String),
    #[error("Failed to read zip archive: {0}")]
    Zip(String),
    #[error("Keychain operation failed: {0}")]
    Keychain(String),
    #[error("Network request failed: {0}")]
    Network(String),
    #[error("Apple authentication failed: {0}")]
    AppleAuth(String),
    #[error("ADI operation failed: {0}")]
    Adi(String),
    #[error("Device discovery failed: {0}")]
    DeviceDiscovery(String),
    #[error("Device installation failed: {0}")]
    DeviceInstall(String),
    #[error("Cache operation failed: {0}")]
    Cache(String),
    #[error("Preferences operation failed: {0}")]
    Preferences(String),
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
    #[error("{0}")]
    Message(String),
    #[error("Backend task for {label} was canceled before it completed.")]
    TaskCanceled { label: &'static str },
}

impl BackendError {
    pub(crate) fn user_message(&self) -> String {
        self.to_string()
    }
}

impl From<String> for BackendError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for BackendError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_preserves_backend_context() {
        assert_eq!(
            BackendError::from("Network request failed").user_message(),
            "Network request failed"
        );
        assert_eq!(
            BackendError::TaskCanceled {
                label: "developer login"
            }
            .user_message(),
            "Backend task for developer login was canceled before it completed."
        );
    }

    #[test]
    fn subsystem_errors_have_ui_messages() {
        let path = std::path::PathBuf::from("/tmp/settings.toml");
        let cases = vec![
            BackendError::Io {
                action: "Read settings",
                path: path.clone(),
                source: std::io::Error::other("denied"),
            }
            .user_message(),
            BackendError::Command {
                action: "Open folder",
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
            }
            .user_message(),
            BackendError::CommandFailed {
                action: "generate CSR",
                detail: "bad key".to_string(),
            }
            .user_message(),
            BackendError::Plist("bad plist".to_string()).user_message(),
            BackendError::Zip("bad zip".to_string()).user_message(),
            BackendError::Keychain("locked".to_string()).user_message(),
            BackendError::Network("offline".to_string()).user_message(),
            BackendError::AppleAuth("invalid password".to_string()).user_message(),
            BackendError::Adi("not provisioned".to_string()).user_message(),
            BackendError::DeviceDiscovery("usbmuxd stopped".to_string()).user_message(),
            BackendError::DeviceInstall("verification failed".to_string()).user_message(),
            BackendError::Cache("cache corrupt".to_string()).user_message(),
            BackendError::Preferences("settings corrupt".to_string()).user_message(),
            BackendError::Unsupported("not on this platform".to_string()).user_message(),
            BackendError::TaskCanceled {
                label: "IPA reader",
            }
            .user_message(),
        ];

        for message in cases {
            assert!(!message.trim().is_empty());
        }
    }
}
