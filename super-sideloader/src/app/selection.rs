use crate::app::models::AppOption;
use crate::app::AppResult;
use std::path::Path;

#[derive(Debug, Default)]
pub(crate) struct AppSelection {
    apps: Vec<AppOption>,
    selected: usize,
    loading_path: Option<String>,
    error_path: Option<String>,
    load_error: Option<String>,
}

impl AppSelection {
    pub(crate) fn selected(&self) -> Option<&AppOption> {
        self.apps.get(self.selected)
    }

    pub(crate) fn app(&self, index: usize) -> Option<&AppOption> {
        self.apps.get(index)
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
        let Some(index) = self.apps.iter().position(|app| app.path.as_str() == path) else {
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

    pub(crate) fn finish_loading(&mut self, path: &Path, result: AppResult<AppOption>) {
        match result {
            Ok(app) => self.push_loaded(app),
            Err(error) => {
                let message = error.user_message();
                log::warn!("{message}");
                self.fail(path.to_string_lossy().to_string(), message);
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

pub(crate) fn is_ipa_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ipa"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::models::{AppMetadata, EntitlementsSource, PatchOption, SupportedDeviceFamily};
    use crate::app::AppError;

    fn sample_app(path: &str, name: &str) -> AppOption {
        AppOption {
            metadata: AppMetadata::sample(
                name,
                "com.example.app",
                "1.0",
                "1",
                name,
                "16.0",
                vec![SupportedDeviceFamily::IPhone],
            ),
            path: path.to_string(),
            icon_path: None,
            icon_override_path: None,
            entitlements: Vec::new(),
            entitlements_source: EntitlementsSource::GeneratedFallback,
            entitlement_overrides: None,
            patches: vec![PatchOption {
                name: "Patch".to_string(),
                detail: "Detail".to_string(),
            }],
        }
    }

    #[test]
    fn ipa_paths_are_matched_case_insensitively() {
        assert!(is_ipa_path(Path::new("/tmp/App.IPA")));
        assert!(is_ipa_path(Path::new("/tmp/App.ipa")));
        assert!(!is_ipa_path(Path::new("/tmp/App.zip")));
        assert!(!is_ipa_path(Path::new("/tmp/App")));
    }

    #[test]
    fn failed_load_keeps_path_for_preferences_and_visible_error() {
        let mut selection = AppSelection::default();
        selection.begin_loading("/tmp/App.ipa".to_string());
        selection.finish_loading(
            Path::new("/tmp/App.ipa"),
            Err(AppError::from("Invalid IPA")),
        );

        assert_eq!(
            selection.selected_path_for_preferences().as_deref(),
            Some("/tmp/App.ipa")
        );
        assert_eq!(selection.error_path(), Some("/tmp/App.ipa"));
        assert_eq!(selection.load_error(), Some("Invalid IPA"));
        assert!(!selection.is_loading());
    }

    #[test]
    fn successful_load_selects_app_and_clears_transient_state() {
        let mut selection = AppSelection::default();
        selection.begin_loading("/tmp/App.ipa".to_string());
        selection.finish_loading(
            Path::new("/tmp/App.ipa"),
            Ok(sample_app("/tmp/App.ipa", "App")),
        );

        assert_eq!(selection.selected_index(), 0);
        assert_eq!(
            selection.selected_path_for_preferences().as_deref(),
            Some("/tmp/App.ipa")
        );
        assert_eq!(
            selection.selected().map(|app| app.name().as_str()),
            Some("App")
        );
        assert!(!selection.is_loading());
        assert_eq!(selection.error_path(), None);
        assert_eq!(selection.load_error(), None);
    }

    #[test]
    fn replacement_updates_existing_app_without_changing_selection() {
        let mut selection = AppSelection::default();
        selection.finish_loading(
            Path::new("/tmp/Old.ipa"),
            Ok(sample_app("/tmp/Old.ipa", "Old")),
        );

        assert!(selection.replace(0, sample_app("/tmp/New.ipa", "New")));

        assert_eq!(selection.selected_index(), 0);
        assert_eq!(
            selection.selected_path_for_preferences().as_deref(),
            Some("/tmp/New.ipa")
        );
        assert_eq!(
            selection.selected().map(|app| app.name().as_str()),
            Some("New")
        );
        assert!(!selection.replace(10, sample_app("/tmp/Missing.ipa", "Missing")));
    }
}
