use crate::ipa::read_ipa;
use crate::models::{AppOption, PatchOption};
use crate::preferences::{apply_app_overrides, AppPreferences};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub(crate) struct AppSelection {
    apps: Vec<AppOption>,
    selected: usize,
    loading_path: Option<String>,
    error_path: Option<String>,
    load_error: Option<String>,
}

impl AppSelection {
    pub(crate) fn from_preferences(preferences: &AppPreferences) -> Self {
        let mut selection = Self::default();
        let Some(path) = preferences.path.as_deref() else {
            return selection;
        };

        if !is_ipa_path(Path::new(path)) {
            selection.fail(
                path.to_string(),
                format!("Selected app path is not an IPA archive: {path}"),
            );
            return selection;
        }

        match load_ipa(PathBuf::from(path), Vec::new()) {
            Ok(mut app) => {
                apply_app_overrides(&mut app, &preferences.overrides);
                selection.push_loaded(app);
            }
            Err(error) => {
                eprintln!("{error}");
                selection.fail(path.to_string(), error);
            }
        }

        selection
    }

    pub(crate) fn selected(&self) -> Option<&AppOption> {
        self.apps.get(self.selected)
    }

    pub(crate) fn selected_index(&self) -> usize {
        self.selected
    }

    pub(crate) fn selected_path_for_preferences(&self) -> Option<String> {
        self.selected()
            .map(|app| app.path.to_string())
            .or_else(|| self.error_path.clone())
            .or_else(|| self.loading_path.clone())
    }

    pub(crate) fn is_loading(&self) -> bool {
        self.loading_path.is_some()
    }

    pub(crate) fn loading_path(&self) -> Option<&str> {
        self.loading_path.as_deref()
    }

    pub(crate) fn error_path(&self) -> Option<&str> {
        self.error_path.as_deref()
    }

    pub(crate) fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    pub(crate) fn replace(&mut self, index: usize, app: AppOption) -> bool {
        let Some(existing) = self.apps.get_mut(index) else {
            return false;
        };
        *existing = app;
        true
    }

    pub(crate) fn select_cached_path(&mut self, path: &str) -> bool {
        let Some(index) = self.apps.iter().position(|app| app.path.as_ref() == path) else {
            return false;
        };

        self.selected = index;
        self.clear_transient_state();
        true
    }

    pub(crate) fn begin_loading(&mut self, path: String) {
        self.loading_path = Some(path);
        self.error_path = None;
        self.load_error = None;
    }

    pub(crate) fn finish_loading(&mut self, path: &Path, result: Result<AppOption, String>) {
        match result {
            Ok(app) => self.push_loaded(app),
            Err(error) => {
                eprintln!("{error}");
                self.fail(path.to_string_lossy().to_string(), error);
            }
        }
    }

    pub(crate) fn fail(&mut self, path: String, error: String) {
        self.selected = self.apps.len();
        self.loading_path = None;
        self.error_path = Some(path);
        self.load_error = Some(error);
    }

    fn push_loaded(&mut self, app: AppOption) {
        self.apps.push(app);
        self.selected = self.apps.len() - 1;
        self.clear_transient_state();
    }

    fn clear_transient_state(&mut self) {
        self.loading_path = None;
        self.error_path = None;
        self.load_error = None;
    }
}

pub(crate) fn load_ipa(path: PathBuf, patches: Vec<PatchOption>) -> Result<AppOption, String> {
    read_ipa(&path, patches)
}

pub(crate) fn is_ipa_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ipa"))
}
