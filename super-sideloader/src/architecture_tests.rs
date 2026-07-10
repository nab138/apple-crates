use std::fs;
use std::path::{Path, PathBuf};

fn assert_absent(module: &str, source: &str, forbidden: &[&str]) {
    for pattern in forbidden {
        assert!(
            !source.contains(pattern),
            "{module} must not contain `{pattern}`"
        );
    }
}

fn rust_files_under(relative_dir: &str) -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_dir);
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

fn collect_rust_files(dir: &Path, files: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", dir.display());
    }) {
        let entry = entry.expect("failed to read source directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let source = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", path.display());
            });
            files.push((display_path(&path), source));
        }
    }
}

fn display_path(path: &Path) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.strip_prefix(&manifest_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn settings_rs() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/settings.rs"))
        .expect("settings source should be readable")
}

#[test]
fn ui_modules_do_not_import_backend_modules_directly() {
    let forbidden = [
        "crate::backend",
        "crate::developer_repository",
        "crate::developer_cache",
        "crate::developer_client",
        "crate::developer_keychain",
        "crate::ipa",
        "crate::paths",
        "std::fs",
        "process::Command",
    ];

    for (module, source) in rust_files_under("src/ui") {
        assert_absent(&module, &source, &forbidden);
    }
}

#[test]
fn render_modules_do_not_update_parent_directly() {
    for (module, source) in rust_files_under("src/ui/settings") {
        assert_absent(&module, &source, &["parent.update", ".parent.update"]);
    }
}

#[test]
fn settings_mode_is_lightweight() {
    let source = settings_rs();
    let mode_source = source
        .split_once("pub(crate) enum SettingsMode")
        .and_then(|(_, after_mode_start)| {
            after_mode_start.split_once("struct DeveloperLoginSnapshot")
        })
        .map(|(mode_source, _)| mode_source)
        .expect("SettingsMode source section should be present");

    assert_absent(
        "ui/settings.rs SettingsMode",
        mode_source,
        &[
            "theme_preference",
            "adi_backend",
            "machine_identity",
            "android_adi_identifier",
            "teams",
            "selected_team",
            "selected_certificate",
            "auto_app_id",
            "selected_app_id",
            "app:",
            "enabled_patches",
            "team_id",
            "backends",
            "selected_backend",
            "WeakEntity",
            "parent:",
        ],
    );
}

#[test]
fn settings_parent_mutations_go_through_dispatcher() {
    let source = settings_rs();
    let (before_dispatcher, dispatcher_and_after) = source
        .split_once("    fn dispatch_parent_action(")
        .expect("settings dispatcher should be present");
    let (_, after_dispatcher) = dispatcher_and_after
        .split_once("    fn selected_developer_context(")
        .expect("settings dispatcher should end before parent query helpers");
    let outside_dispatcher =
        format!("{before_dispatcher}    fn selected_developer_context({after_dispatcher}");

    assert_absent(
        "ui/settings.rs outside dispatch_parent_action",
        &outside_dispatcher,
        &[
            "view.add_developer_account_from_settings",
            "view.select_team_from_settings",
            "view.select_certificate_from_settings",
            "view.select_app_id_from_settings",
            "view.set_auto_app_id_from_settings",
            "view.replace_developer_account_from_settings",
            "view.log_out_selected_developer_account_from_settings",
            "view.set_theme_preference_from_settings",
            "view.select_adi_backend_from_settings",
            "view.replace_adi_backends_from_settings",
            "view.replace_android_device_identity_from_settings",
            "view.replace_app_from_settings",
        ],
    );
}

#[test]
fn settings_render_does_not_access_parent_entity() {
    let source = settings_rs();
    let render_source = source
        .split_once("impl Render for SettingsWindow")
        .and_then(|(_, after_render_start)| {
            after_render_start.split_once("pub(crate) fn show_or_open_settings_window")
        })
        .map(|(render_source, _)| render_source)
        .expect("settings render source section should be present");

    assert_absent(
        "ui/settings.rs SettingsWindow::render",
        render_source,
        &[
            "parent.update",
            "parent.read_with",
            ".parent.update",
            ".parent.read_with",
        ],
    );
}

#[test]
fn backend_and_domain_modules_do_not_import_gpui_ui_or_app_models() {
    let forbidden = [
        "gpui",
        "gpui_component",
        "crate::ui",
        "crate::main_view",
        "crate::settings",
        "crate::widgets",
        "crate::app::models",
    ];

    for (module, source) in rust_files_under("src/backend")
        .into_iter()
        .chain(rust_files_under("src/domain"))
    {
        assert_absent(&module, &source, &forbidden);
    }
}

#[test]
fn backend_services_return_domain_types_not_app_models() {
    let service_modules = [
        "src/backend/adi.rs",
        "src/backend/adi_service.rs",
        "src/backend/developer/service.rs",
        "src/backend/device.rs",
        "src/backend/ipa.rs",
        "src/backend/system_identity.rs",
    ];

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    for module in service_modules {
        let source = fs::read_to_string(manifest_dir.join(module))
            .unwrap_or_else(|error| panic!("failed to read {module}: {error}"));
        assert_absent(module, &source, &["crate::app::models"]);
    }
}

#[test]
fn app_models_are_layer_clean() {
    let source =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app/models.rs"))
            .expect("app models source should be readable");
    assert_absent(
        "app/models.rs",
        &source,
        &[
            "crate::backend",
            "crate::ui",
            "gpui",
            "gpui_component",
            "WeakEntity",
            "Context<",
            "ClickEvent",
        ],
    );
}

#[test]
fn app_layer_does_not_import_ui_or_gpui() {
    for (module, source) in rust_files_under("src/app") {
        assert_absent(&module, &source, &["crate::ui", "gpui", "gpui_component"]);
    }
}

#[test]
fn only_shared_backend_runtime_builds_tokio_runtimes() {
    let forbidden = ["tokio::runtime::Builder", ".block_on("];

    for (module, source) in rust_files_under("src") {
        if module == "src/backend/runtime.rs" || module == "src/architecture_tests.rs" {
            continue;
        }
        assert_absent(&module, &source, &forbidden);
    }
}

#[test]
fn backend_modules_do_not_use_blocking_reqwest() {
    for (module, source) in rust_files_under("src/backend") {
        assert_absent(&module, &source, &["reqwest::blocking"]);
    }
}
