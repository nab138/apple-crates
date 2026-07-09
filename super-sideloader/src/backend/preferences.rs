use crate::backend::paths::app_data_dir;
use crate::backend::{BackendError, BackendResult};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

const SETTINGS_FILE: &str = "settings.toml";

pub(crate) fn load_settings_toml() -> BackendResult<Option<String>> {
    let path = settings_path()?;
    match fs::read_to_string(&path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(BackendError::Io {
            action: "Read settings",
            path,
            source,
        }),
    }
}

pub(crate) fn save_settings_toml(contents: &str) -> BackendResult<()> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| BackendError::Io {
            action: "Create settings folder",
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(&path, contents).map_err(|source| BackendError::Io {
        action: "Write settings",
        path,
        source,
    })
}

fn settings_path() -> BackendResult<PathBuf> {
    app_data_dir()
        .map(|path| path.join(SETTINGS_FILE))
        .ok_or_else(|| {
            BackendError::Preferences("The application data folder is not available.".to_string())
        })
}
