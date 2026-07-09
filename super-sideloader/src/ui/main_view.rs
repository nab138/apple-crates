use crate::app::effects as app_effects;
use crate::app::models::{
    AccountOption, AdiBackendOption, AppOption, DeviceOption, MachineIdentity, PickerId,
    SideloadOperation, SideloadPhase,
};
use crate::app::preferences::{apply_app_overrides, AppOverridePreferences};
use crate::app::state::SideloaderState;
use crate::app::AppError;
use crate::constants::*;
use crate::ui::settings::{show_or_open_settings_window, SettingsMode, SettingsWindowHandle};
use crate::ui::theme::{fixed_rgb, rgb, sync_window_theme};
use crate::ui::widgets::{
    app_identity, connector_arrow_icon, device_identity, dropdown_list, floating_menu_under,
    icon_button_surface, lucide_icon, lucide_icon_tinted, progress_circle, properties_list,
    select_action_row, select_button, select_item_content, select_option_button,
    select_with_popover, square_button_surface, surface_button,
};
use futures::{channel::mpsc, StreamExt};
use gpui::{
    div, prelude::*, px, App, ClickEvent, Context, ExternalPaths, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, ParentElement, PathPromptOptions, PromptButton, PromptLevel,
    Render, Styled, Window,
};
use gpui_component::button::Button;
use gpui_component::text::TextView;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

const SIDELOAD_SIGNING_WEIGHT: f32 = 0.72;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SigningAction {
    Save,
    Sideload,
}

impl SigningAction {
    fn installs_after_signing(self) -> bool {
        self == Self::Sideload
    }

    fn confirmation_label(self) -> &'static str {
        match self {
            Self::Save => "Create and Sign",
            Self::Sideload => "Create and Sideload",
        }
    }
}

pub(crate) struct SideloaderView {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) state: SideloaderState,
    pub(crate) open_picker: Option<PickerId>,
    pub(crate) spinner_turns: f32,
    pub(crate) team_settings_window: Option<SettingsWindowHandle>,
    pub(crate) app_settings_window: Option<SettingsWindowHandle>,
    pub(crate) adi_settings_window: Option<SettingsWindowHandle>,
}

impl Deref for SideloaderView {
    type Target = SideloaderState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for SideloaderView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl SideloaderView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        cx.on_release(|view, cx| view.close_child_windows(cx))
            .detach();
        cx.observe_window_appearance(window, |view, window, cx| {
            sync_window_theme(window, cx, view.theme_preference);
            cx.notify();
        })
        .detach();

        let loaded_state = SideloaderState::load();
        let app_path_to_restore = loaded_state.app_path_to_restore;
        let app_overrides_to_restore = loaded_state.app_overrides_to_restore;
        let should_save_preferences = loaded_state.should_save_preferences;

        let mut view = Self {
            focus_handle,
            state: loaded_state.state,
            open_picker: None,
            spinner_turns: 0.,
            team_settings_window: None,
            app_settings_window: None,
            adi_settings_window: None,
        };
        if let Some(path) = app_path_to_restore {
            view.select_ipa_path_with_overrides(path, Some(app_overrides_to_restore), cx);
        } else if should_save_preferences {
            view.save_preferences();
        }
        view.start_device_refresh(cx);
        view.start_device_events_watch(cx);
        view
    }

    fn close_child_windows(&mut self, cx: &mut App) {
        close_settings_window(self.team_settings_window.take(), cx);
        close_settings_window(self.app_settings_window.take(), cx);
        close_settings_window(self.adi_settings_window.take(), cx);
    }

    fn sync_settings_windows(&self, cx: &mut Context<Self>) {
        self.sync_developer_settings_window(cx);
        self.sync_app_settings_window(cx);
        self.sync_adi_settings_window(cx);
    }

    fn sync_developer_settings_window(&self, cx: &mut Context<Self>) {
        if let Some(handle) = self.team_settings_window.as_ref() {
            handle.sync_from_state(&self.state, cx);
        }
    }

    fn sync_app_settings_window(&self, cx: &mut Context<Self>) {
        if let Some(handle) = self.app_settings_window.as_ref() {
            handle.sync_from_state(&self.state, cx);
        }
    }

    fn sync_adi_settings_window(&self, cx: &mut Context<Self>) {
        if let Some(handle) = self.adi_settings_window.as_ref() {
            handle.sync_from_state(&self.state, cx);
        }
    }

    pub(crate) fn select_team_from_settings(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let team_id = self.state.select_team(index)?;
        self.save_preferences();
        self.sync_app_settings_window(cx);
        cx.notify();
        Some(team_id)
    }

    pub(crate) fn set_auto_app_id_from_settings(
        &mut self,
        auto_app_id: bool,
        cx: &mut Context<Self>,
    ) {
        self.auto_app_id = auto_app_id;
        self.save_preferences();
        cx.notify();
    }

    pub(crate) fn select_certificate_from_settings(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.selected_certificate = index;
        self.save_preferences();
        cx.notify();
    }

    pub(crate) fn select_app_id_from_settings(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected_app_id = index;
        self.save_preferences();
        cx.notify();
    }

    pub(crate) fn replace_developer_account_from_settings(
        &mut self,
        account: AccountOption,
        cx: &mut Context<Self>,
    ) -> Option<SettingsMode> {
        if !self
            .state
            .replace_developer_account_preserving_selection(account)
        {
            return None;
        }
        self.save_preferences();
        self.sync_app_settings_window(cx);
        cx.notify();
        self.accounts
            .get(self.selected_account)
            .map(|_| SettingsMode::Team)
    }

    pub(crate) fn add_developer_account_from_settings(
        &mut self,
        account: AccountOption,
        cx: &mut Context<Self>,
    ) -> Option<SettingsMode> {
        self.state.add_developer_account(account);
        self.open_picker = None;
        self.save_preferences();
        self.sync_app_settings_window(cx);
        cx.notify();

        self.accounts
            .get(self.selected_account)
            .map(|_| SettingsMode::Team)
    }

    pub(crate) fn log_out_selected_developer_account_from_settings(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<SettingsMode> {
        self.state.log_out_selected_developer_account();
        self.open_picker = None;
        self.save_preferences();
        self.sync_app_settings_window(cx);
        cx.notify();

        let Some(account) = self.accounts.get(self.selected_account) else {
            return Some(self.developer_login_request());
        };
        if account.teams.is_empty() {
            return Some(self.developer_login_request());
        }

        let _ = account;
        Some(SettingsMode::Team)
    }

    pub(crate) fn select_adi_backend_from_settings(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        if index < self.adi_backends.len() {
            self.selected_adi_backend = index;
            self.save_preferences();
            self.sync_developer_settings_window(cx);
            cx.notify();
        }
    }

    pub(crate) fn replace_app_from_settings(
        &mut self,
        app_index: usize,
        app: AppOption,
        cx: &mut Context<Self>,
    ) {
        if self.state.replace_app(app_index, app) {
            self.save_preferences();
            cx.notify();
        }
    }

    pub(crate) fn replace_adi_backends_from_settings(
        &mut self,
        backends: Vec<AdiBackendOption>,
        selected_backend: usize,
        persist_selection: bool,
        cx: &mut Context<Self>,
    ) {
        self.state.replace_adi_backends(backends, selected_backend);
        if persist_selection {
            self.save_preferences();
        }
        self.sync_developer_settings_window(cx);
        cx.notify();
    }

    pub(crate) fn replace_android_device_identity_from_settings(
        &mut self,
        identity: MachineIdentity,
        cx: &mut Context<Self>,
    ) {
        self.state.replace_android_device_identity(identity);
        self.save_preferences();
        self.sync_developer_settings_window(cx);
        cx.notify();
    }

    fn developer_login_request(&self) -> SettingsMode {
        SettingsMode::DeveloperLogin
    }

    fn select_account(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        if index < self.accounts.len() {
            self.selected_account = index;
            self.selected_team = 0;
            self.selected_certificate = 0;
            self.selected_app_id = 0;
            self.open_picker = None;
            self.save_preferences();
            self.sync_settings_windows(cx);
            cx.notify();
        }
    }

    fn select_device(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        if self.device_selection.select(index) {
            self.open_picker = None;
            self.save_preferences();
            self.sync_developer_settings_window(cx);
            cx.notify();
        }
    }

    fn manage_accounts(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        self.open_picker = None;
        let parent = cx.weak_entity();
        let request = self.developer_login_request();
        self.team_settings_window = Some(show_or_open_settings_window(
            self.team_settings_window.take(),
            parent,
            request,
            &self.state,
            SETTINGS_WINDOW_WIDTH,
            TEAM_SETTINGS_WINDOW_HEIGHT,
            cx,
        ));
        cx.notify();
    }

    fn choose_ipa(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        self.open_picker = None;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select IPA".into()),
        });

        cx.spawn(async move |view, cx| {
            let paths = match receiver.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    log::warn!("Failed to select IPA: {error}");
                    let _ = view.update(cx, |view, cx| {
                        view.app_selection.fail(
                            "Select IPA".to_string(),
                            format!("Failed to select IPA: {error}"),
                        );
                        view.sync_app_settings_window(cx);
                        cx.notify();
                    });
                    return;
                }
            };

            let Some(path) = paths.into_iter().next() else {
                return;
            };

            let _ = view.update(cx, |view, cx| {
                view.select_ipa_path(path, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn drop_ipa(&mut self, paths: &ExternalPaths, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.stop_propagation();
        if self.is_busy() {
            return;
        }
        if let Some(path) = first_ipa_path(paths) {
            self.select_ipa_path(path, cx);
        }
    }

    fn select_ipa_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.select_ipa_path_with_overrides(path, None, cx);
    }

    fn select_ipa_path_with_overrides(
        &mut self,
        path: PathBuf,
        overrides: Option<AppOverridePreferences>,
        cx: &mut Context<Self>,
    ) {
        if !app_effects::is_ipa_path(&path) {
            let path = path.to_string_lossy().to_string();
            self.app_selection.fail(
                path.clone(),
                format!("Selected file is not an IPA archive: {path}"),
            );
            self.enabled_patches.clear();
            self.save_preferences();
            self.sync_app_settings_window(cx);
            cx.notify();
            return;
        }

        let path_string = path.to_string_lossy().to_string();
        if self.app_selection.select_cached_path(&path_string) {
            self.enabled_patches = self
                .selected_app()
                .map(|app| vec![false; app.patches.len()])
                .unwrap_or_default();
            self.open_picker = None;
            self.save_preferences();
            self.sync_app_settings_window(cx);
            cx.notify();
            return;
        }

        let patches = self
            .selected_app()
            .map(|app| app.patches.clone())
            .unwrap_or_default();
        self.app_selection.begin_loading(path_string);
        self.open_picker = None;
        cx.notify();

        cx.spawn(async move |view, cx| {
            let path_for_error = path.clone();
            let result = cx
                .background_spawn(async move {
                    let mut app = app_effects::load_ipa(path, patches.clone()).await?;
                    if let Some(overrides) = overrides.as_ref() {
                        apply_app_overrides(&mut app, overrides);
                    }
                    Ok::<_, AppError>(app)
                })
                .await;

            let _ = view.update(cx, |view, cx| {
                view.app_selection.finish_loading(&path_for_error, result);
                view.enabled_patches = view
                    .selected_app()
                    .map(|app| vec![false; app.patches.len()])
                    .unwrap_or_default();
                view.save_preferences();
                view.sync_app_settings_window(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn sideload(&mut self, event: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.start_signing(SigningAction::Sideload, event, window, cx);
    }

    fn sign(&mut self, event: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.start_signing(SigningAction::Save, event, window, cx);
    }

    fn start_signing(
        &mut self,
        action: SigningAction,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy()
            || self.selected_account().is_none()
            || self.selected_app().is_none()
            || (action.installs_after_signing() && self.selected_device().is_none())
        {
            return;
        }

        let Some(account) = self.selected_account().cloned() else {
            return;
        };
        let Some(team) = account.teams.get(self.selected_team).cloned() else {
            self.fail_signing_and_open_developer_settings(
                "Select a developer team before signing.",
                event,
                window,
                cx,
            );
            return;
        };
        let Some(certificate) = team.certificates.get(self.selected_certificate).cloned() else {
            self.fail_signing_and_open_developer_settings(
                "Create a development certificate in Developer Settings before signing.",
                event,
                window,
                cx,
            );
            return;
        };
        if !certificate.private_key_available {
            self.fail_signing_and_open_developer_settings(
                "Import the selected certificate's PEM private key in Developer Settings before signing.",
                event,
                window,
                cx,
            );
            return;
        }
        let Some(certificate_fingerprint) = certificate.certificate_fingerprint.clone() else {
            self.fail_signing_and_open_developer_settings(
                "Refresh Developer Settings to cache the selected certificate before signing.",
                event,
                window,
                cx,
            );
            return;
        };
        let Some(public_key_fingerprint) = certificate.public_key_fingerprint.clone() else {
            self.fail_signing_and_open_developer_settings(
                "Refresh Developer Settings to inspect the selected certificate before signing.",
                event,
                window,
                cx,
            );
            return;
        };
        let selected_app_id = team.app_ids.get(self.selected_app_id).cloned();
        if !self.auto_app_id && selected_app_id.is_none() {
            self.fail_signing_and_open_developer_settings(
                "Select an App ID in Developer Settings before signing.",
                event,
                window,
                cx,
            );
            return;
        }
        let developer_context = match self.selected_developer_context() {
            Ok(context) => context,
            Err(error) => {
                self.sideload_operation = SideloadOperation::Failed { message: error };
                cx.notify();
                return;
            }
        };
        let app = self
            .selected_app()
            .cloned()
            .expect("selected app checked above");
        let selected_device = action
            .installs_after_signing()
            .then(|| self.selected_device().cloned())
            .flatten();
        let app_name = app.name().to_string();
        let (directory, suggested_name) = signed_ipa_destination(&app.path);
        let destination_receiver = if action == SigningAction::Save {
            Some(cx.prompt_for_new_path(&directory, Some(&suggested_name)))
        } else {
            None
        };
        let auto_app_id = self.auto_app_id;
        let team_id = team.identifier;
        let selected_app_id_identifier = selected_app_id
            .as_ref()
            .map(|app_id| app_id.identifier.clone());
        self.open_picker = None;
        let (progress_sender, mut progress_receiver) = mpsc::unbounded();
        cx.spawn(async move |view, cx| {
            while let Some(event) = progress_receiver.next().await {
                let _ = view.update(cx, |view, cx| {
                    view.set_signing_progress(event, action.installs_after_signing());
                    cx.notify();
                });
            }
        })
        .detach();
        let (install_progress_sender, mut install_progress_receiver) = mpsc::unbounded();
        cx.spawn(async move |view, cx| {
            while let Some(event) = install_progress_receiver.next().await {
                let _ = view.update(cx, |view, cx| {
                    view.set_installing_progress(event);
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();

        cx.spawn_in(window, async move |view, cx| {
            let signing_output = if let Some(receiver) = destination_receiver {
                match receiver.await {
                    Ok(Ok(Some(path))) => app_effects::SignIpaOutput::Ipa(path),
                    Ok(Ok(None)) | Err(_) => return,
                    Ok(Err(error)) => {
                        let _ = view.update(cx, |view, cx| {
                            view.sideload_operation = SideloadOperation::Failed {
                                message: format!(
                                    "Failed to choose a signed IPA destination: {error}"
                                ),
                            };
                            cx.notify();
                        });
                        return;
                    }
                }
            } else {
                app_effects::SignIpaOutput::AppBundle
            };

            let _ = view.update(cx, |view, cx| {
                view.sideload_operation = SideloadOperation::Running {
                    phase: SideloadPhase::Signing,
                    progress: 0.02,
                    detail: "Refreshing App IDs".to_string(),
                };
                cx.notify();
            });

            let refreshed_account = match cx
                .background_spawn(app_effects::refresh_account(developer_context.clone()))
                .await
            {
                Ok(account) => account,
                Err(error) => {
                    let _ = view.update(cx, |view, cx| {
                        view.sideload_operation = SideloadOperation::Failed {
                            message: format!(
                                "Could not refresh App IDs before signing: {}",
                                error.user_message()
                            ),
                        };
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(refreshed_team) = refreshed_account
                .teams
                .iter()
                .find(|team| team.identifier == team_id)
                .cloned()
            else {
                let _ = view.update(cx, |view, cx| {
                    view.sideload_operation = SideloadOperation::Failed {
                        message: "The selected developer team was not returned by Apple."
                            .to_string(),
                    };
                    cx.notify();
                });
                return;
            };
            if let Some(device) = selected_device.as_ref() {
                let _ = view.update(cx, |view, cx| {
                    view.sideload_operation = SideloadOperation::Running {
                        phase: SideloadPhase::Signing,
                        progress: 0.03,
                        detail: "Checking device registration".to_string(),
                    };
                    cx.notify();
                });
                let registered_devices = match cx
                    .background_spawn(app_effects::list_developer_devices(
                        developer_context.clone(),
                        team_id.clone(),
                    ))
                    .await
                {
                    Ok(devices) => devices,
                    Err(error) => {
                        let _ = view.update(cx, |view, cx| {
                            view.sideload_operation = SideloadOperation::Failed {
                                message: format!(
                                    "Could not check whether {} is registered with Apple: {}",
                                    device.name,
                                    error.user_message()
                                ),
                            };
                            cx.notify();
                        });
                        return;
                    }
                };
                let device_is_registered = registered_devices.iter().any(|registered| {
                    registered
                        .udid
                        .trim()
                        .eq_ignore_ascii_case(device.udid.trim())
                });
                if !device_is_registered {
                    let _ = view.update(cx, |view, cx| {
                        view.sideload_operation = SideloadOperation::Running {
                            phase: SideloadPhase::Signing,
                            progress: 0.035,
                            detail: format!("Registering {} with Apple", device.name),
                        };
                        cx.notify();
                    });
                    if let Err(error) = cx
                        .background_spawn(app_effects::add_developer_device(
                            developer_context.clone(),
                            team_id.clone(),
                            device.name.clone(),
                            device.udid.clone(),
                        ))
                        .await
                    {
                        let _ = view.update(cx, |view, cx| {
                            view.sideload_operation = SideloadOperation::Failed {
                                message: format!(
                                    "Could not register {} with Apple: {}",
                                    device.name,
                                    error.user_message()
                                ),
                            };
                            cx.notify();
                        });
                        return;
                    }
                }
            }
            let refreshed_selected_app_id = selected_app_id_identifier.as_deref().and_then(|id| {
                refreshed_team
                    .app_ids
                    .iter()
                    .find(|app_id| app_id.identifier == id)
                    .cloned()
            });
            let plan = match app_effects::app_id_provisioning_plan(
                &refreshed_team,
                auto_app_id,
                refreshed_selected_app_id.clone(),
                &app,
            ) {
                Ok(plan) => plan,
                Err(error) => {
                    let _ = view.update(cx, |view, cx| {
                        view.state
                            .replace_developer_account_preserving_selection(refreshed_account);
                        view.save_preferences();
                        view.sync_settings_windows(cx);
                        view.sideload_operation = SideloadOperation::Failed {
                            message: error.user_message(),
                        };
                        cx.notify();
                    });
                    return;
                }
            };

            let _ = view.update(cx, |view, cx| {
                view.state
                    .replace_developer_account_preserving_selection(refreshed_account);
                view.save_preferences();
                view.sync_settings_windows(cx);
                cx.notify();
            });

            if plan.app_ids.len() > 1 {
                let (message, detail) = app_id_provisioning_prompt(&plan);
                let prompt = match view.update_in(cx, |_, window, cx| {
                    window.prompt(
                        PromptLevel::Warning,
                        &message,
                        Some(&detail),
                        &[
                            PromptButton::new(action.confirmation_label()),
                            PromptButton::Cancel("Cancel".into()),
                        ],
                        cx,
                    )
                }) {
                    Ok(prompt) => prompt,
                    Err(_) => return,
                };
                if prompt.await != Ok(0) {
                    let _ = view.update(cx, |view, cx| {
                        view.sideload_operation = SideloadOperation::Idle;
                        cx.notify();
                    });
                    return;
                }
            }

            let _ = view.update(cx, |view, cx| {
                view.sideload_operation = SideloadOperation::Running {
                    phase: SideloadPhase::Signing,
                    progress: 0.04,
                    detail: "Preparing signing resources".to_string(),
                };
                cx.notify();
            });
            let request = app_effects::SignIpaRequest {
                developer_context,
                team_id,
                team_app_ids: refreshed_team.app_ids,
                certificate_fingerprint,
                public_key_fingerprint,
                auto_app_id,
                selected_app_id: refreshed_selected_app_id,
                app,
                output: signing_output,
            };
            let result = cx
                .background_spawn(async move {
                    app_effects::sign_ipa(request, move |event| {
                        let _ = progress_sender.unbounded_send(event);
                    })
                    .await
                    .map_err(|error| error.user_message())
                })
                .await;
            let outcome = match result {
                Ok(outcome) => outcome,
                Err(message) => {
                    let _ = view.update(cx, |view, cx| {
                        view.sideload_operation = SideloadOperation::Failed { message };
                        cx.notify();
                    });
                    return;
                }
            };
            let app_effects::SignIpaOutcome {
                artifact,
                updated_account,
            } = outcome;

            let _ = view.update(cx, |view, cx| {
                if let Some(account) = updated_account {
                    view.state
                        .replace_developer_account_preserving_selection(account);
                    view.save_preferences();
                    view.sync_settings_windows(cx);
                }
                cx.notify();
            });
            if action == SigningAction::Save {
                let app_effects::SignIpaArtifact::Ipa(output_path) = artifact else {
                    let _ = view.update(cx, |view, cx| {
                        view.sideload_operation = SideloadOperation::Failed {
                            message: "Signing completed without producing an IPA.".to_string(),
                        };
                        cx.notify();
                    });
                    return;
                };
                let file_name = output_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("signed IPA");
                let _ = view.update(cx, |view, cx| {
                    view.sideload_operation = SideloadOperation::Finished {
                        message: format!("Signed {file_name}"),
                    };
                    cx.notify();
                });
                return;
            }

            let app_effects::SignIpaArtifact::AppBundle(signed_app) = artifact else {
                let _ = view.update(cx, |view, cx| {
                    view.sideload_operation = SideloadOperation::Failed {
                        message: "Signing unexpectedly produced an IPA for sideloading."
                            .to_string(),
                    };
                    cx.notify();
                });
                return;
            };

            let device = selected_device.expect("sideload device selected above");
            let _ = view.update(cx, |view, cx| {
                view.sideload_operation = SideloadOperation::Running {
                    phase: SideloadPhase::Installing,
                    progress: SIDELOAD_SIGNING_WEIGHT,
                    detail: format!("Connecting to {}", device.name),
                };
                cx.notify();
            });
            let install_result = cx
                .background_spawn(app_effects::install_app(
                    device.udid.clone(),
                    signed_app,
                    move |event| {
                        let _ = install_progress_sender.unbounded_send(event);
                    },
                ))
                .await
                .map_err(|error| error.user_message());

            let _ = view.update(cx, |view, cx| {
                view.sideload_operation = match install_result {
                    Ok(()) => SideloadOperation::Finished {
                        message: format!("Installed {app_name} on {}", device.name),
                    },
                    Err(message) => SideloadOperation::Failed { message },
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn fail_signing_and_open_developer_settings(
        &mut self,
        message: &str,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sideload_operation = SideloadOperation::Failed {
            message: message.to_string(),
        };
        self.open_team_settings(event, window, cx);
    }

    fn set_signing_progress(
        &mut self,
        event: app_effects::SignIpaProgress,
        installs_after_signing: bool,
    ) {
        let SideloadOperation::Running {
            phase: SideloadPhase::Signing,
            progress: current_progress,
            ..
        } = &self.sideload_operation
        else {
            return;
        };
        let next_progress = if installs_after_signing {
            event.progress() * SIDELOAD_SIGNING_WEIGHT
        } else {
            event.progress()
        };
        let progress = next_progress.max(*current_progress);
        self.sideload_operation = SideloadOperation::Running {
            phase: SideloadPhase::Signing,
            progress,
            detail: event.label(),
        };
    }

    fn set_installing_progress(&mut self, event: app_effects::InstallAppProgress) {
        let SideloadOperation::Running {
            phase: SideloadPhase::Installing,
            progress: current_progress,
            ..
        } = &self.sideload_operation
        else {
            return;
        };
        let next_progress =
            SIDELOAD_SIGNING_WEIGHT + event.progress() * (1. - SIDELOAD_SIGNING_WEIGHT);
        self.sideload_operation = SideloadOperation::Running {
            phase: SideloadPhase::Installing,
            progress: next_progress.max(*current_progress),
            detail: event.label(),
        };
    }

    fn refresh_devices_from_button(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        self.start_device_events_watch(cx);
        self.start_device_refresh(cx);
    }

    fn open_team_settings(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        let Some(account) = self.selected_account() else {
            return;
        };
        let teams = account.teams.clone();
        if !teams.is_empty() {
            self.selected_team = self.selected_team.min(teams.len().saturating_sub(1));
            self.selected_certificate = self.selected_certificate.min(
                teams[self.selected_team]
                    .certificates
                    .len()
                    .saturating_sub(1),
            );
            self.selected_app_id = self
                .selected_app_id
                .min(teams[self.selected_team].app_ids.len().saturating_sub(1));
        }
        self.open_picker = None;
        let _ = teams;
        self.sync_app_settings_window(cx);
        let request = SettingsMode::Team;
        self.team_settings_window = Some(show_or_open_settings_window(
            self.team_settings_window.take(),
            cx.weak_entity(),
            request,
            &self.state,
            SETTINGS_WINDOW_WIDTH,
            TEAM_SETTINGS_WINDOW_HEIGHT,
            cx,
        ));
        cx.notify();
    }

    fn start_device_refresh(&mut self, cx: &mut Context<Self>) {
        let Some(generation) = self.device_selection.begin_refresh() else {
            return;
        };
        cx.notify();

        cx.spawn(async move |view, cx| {
            let result = cx
                .background_spawn(async move {
                    app_effects::discover_devices()
                        .await
                        .map_err(|error| error.user_message())
                })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.finish_device_refresh(generation, result);
                view.sync_developer_settings_window(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn start_device_events_watch(&mut self, cx: &mut Context<Self>) {
        if !self.device_selection.start_events_watch() {
            return;
        }

        let (sender, mut receiver) = mpsc::unbounded();
        cx.background_spawn(async move {
            app_effects::watch_device_changes(sender)
                .await
                .map_err(|error| error.user_message())
        })
        .detach();

        cx.spawn(async move |view, cx| {
            while let Some(result) = receiver.next().await {
                let should_continue = view
                    .update(cx, |view, cx| match result {
                        app_effects::DeviceWatchEvent::Changed => {
                            view.device_selection.note_device_event();
                            if !view.is_busy() && !view.device_selection.is_refreshing() {
                                view.start_device_refresh(cx);
                            }
                            true
                        }
                        app_effects::DeviceWatchEvent::Failed(error) => {
                            view.device_selection.fail_events_watch(error);
                            cx.notify();
                            false
                        }
                    })
                    .unwrap_or(false);

                if !should_continue {
                    return;
                }
            }

            let _ = view.update(cx, |view, cx| {
                view.device_selection.finish_events_watch();
                cx.notify();
            });
        })
        .detach();
    }

    fn finish_device_refresh(
        &mut self,
        generation: u64,
        result: Result<Vec<DeviceOption>, String>,
    ) {
        self.device_selection.finish_refresh(generation, result);
        if self.device_selection.is_empty() && self.open_picker == Some(PickerId::Device) {
            self.open_picker = None;
        }
    }

    fn open_app_settings(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        let Some(_app) = self.selected_app() else {
            return;
        };
        self.open_picker = None;
        let request = SettingsMode::AppSettings {
            app_index: self.app_selection.selected_index(),
        };
        self.app_settings_window = Some(show_or_open_settings_window(
            self.app_settings_window.take(),
            cx.weak_entity(),
            request,
            &self.state,
            APP_SETTINGS_WINDOW_WIDTH,
            APP_SETTINGS_WINDOW_HEIGHT,
            cx,
        ));
        cx.notify();
    }

    fn open_adi_settings(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        self.open_picker = None;
        self.refresh_adi_backends();
        self.sync_developer_settings_window(cx);
        let request = SettingsMode::AdiSettings;
        self.adi_settings_window = Some(show_or_open_settings_window(
            self.adi_settings_window.take(),
            cx.weak_entity(),
            request,
            &self.state,
            ADI_SETTINGS_WINDOW_WIDTH,
            ADI_SETTINGS_WINDOW_HEIGHT,
            cx,
        ));
        cx.notify();
    }

    fn enabled_patch_names(&self) -> Vec<String> {
        let Some(app) = self.selected_app() else {
            return Vec::new();
        };
        app.patches
            .iter()
            .enumerate()
            .filter_map(|(index, patch)| {
                self.enabled_patches
                    .get(index)
                    .copied()
                    .filter(|enabled| *enabled)
                    .map(|_| patch.name.to_string())
            })
            .collect()
    }

    fn section_header(
        number: &'static str,
        title: &'static str,
        detail: &'static str,
    ) -> gpui::Div {
        Self::section_header_with_action(number, title, detail, div())
    }

    fn section_header_with_action(
        number: &'static str,
        title: &'static str,
        detail: &'static str,
        action: impl IntoElement,
    ) -> gpui::Div {
        div()
            .flex()
            .items_start()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w_6()
                            .h_6()
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(fixed_rgb(0x173f45))
                            .text_color(fixed_rgb(0xffffff))
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(number),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0x24333a))
                                    .child(title),
                            )
                            .child(div().text_xs().text_color(rgb(0x6a7a81)).child(detail)),
                    ),
            )
            .child(action)
    }

    fn settings_button(id: &'static str, label: &'static str, detail: &'static str) -> Button {
        surface_button(id)
            .min_h(px(40.))
            .w_full()
            .cursor_pointer()
            .child(
                div()
                    .min_w_0()
                    .size_full()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .hover(|style| style.bg(rgb(0xeef6f5)))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0x33454c))
                                    .child(label),
                            )
                            .child(div().text_xs().text_color(rgb(0x73858b)).child(detail)),
                    )
                    .child(div().flex_none().w_6().h_6().child(icon_button_surface(
                        "icons/wrench.svg",
                        0xebf1f0,
                        0xdfe8e6,
                        0x53666d,
                    ))),
            )
    }

    fn account_row(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let account = &self.accounts[index];
        let selected = index == self.selected_account;
        let detail = developer_team_count(account);

        select_option_button(
            ("account-row", index),
            selected,
            div()
                .min_w_0()
                .w_full()
                .flex()
                .items_center()
                .child(select_item_content(
                    account.apple_id.clone(),
                    account.label.clone(),
                    detail,
                )),
        )
        .on_click(
            cx.listener(move |this, event, window, cx| {
                this.select_account(index, event, window, cx)
            }),
        )
    }

    fn device_row(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let device = self
            .device_selection
            .device(index)
            .expect("device row index must be valid");
        let selected = index == self.device_selection.selected_index();

        select_option_button(
            ("device-row", index),
            selected,
            div()
                .min_w_0()
                .w_full()
                .flex()
                .items_center()
                .child(select_item_content(
                    device.udid.clone(),
                    device.name.clone(),
                    device_identity(device),
                )),
        )
        .on_click(
            cx.listener(move |this, event, window, cx| {
                this.select_device(index, event, window, cx)
            }),
        )
    }

    fn picker_shell(
        title: impl IntoElement,
        body: impl IntoElement,
        disabled: bool,
    ) -> impl IntoElement {
        div()
            .min_w(px(290.))
            .min_h(px(340.))
            .flex_1()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd8e0df))
            .bg(rgb(0xffffff))
            .when(disabled, |this| this.opacity(0.55))
            .child(title)
            .child(body)
    }

    fn account_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if let Some(account) = self.selected_account() {
            let team = self.selected_team();
            let is_open = self.open_picker == Some(PickerId::Account);
            let detail = developer_team_count(account);
            let mut properties = vec![
                ("Apple Account", account.apple_id.to_string()),
                ("Status", account.status.to_string()),
                ("Source", account.detail.to_string()),
            ];
            if let Some(team) = team {
                properties.push(("Team", team.name.to_string()));
            }

            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(select_with_popover(
                    "account-popover-scroll",
                    select_button(
                        "account-select",
                        account.apple_id.clone(),
                        account.label.clone(),
                        detail,
                        is_open,
                    ),
                    is_open,
                    cx.listener(|this, open: &bool, _, cx| {
                        if this.is_busy() {
                            this.open_picker = None;
                        } else {
                            this.open_picker = (*open).then_some(PickerId::Account);
                        }
                        cx.notify();
                    }),
                    dropdown_list(
                        (0..self.accounts.len()).map(|index| self.account_row(index, cx)),
                    )
                    .child(
                        select_action_row(
                            "manage-accounts",
                            "Add an Apple Account",
                            "Log in to another developer Apple Account.",
                        )
                        .on_click(cx.listener(Self::manage_accounts)),
                    )
                    .id("account-options"),
                ))
                .child(properties_list(properties))
                .child(
                    Self::settings_button(
                        "team-disclosure",
                        "Advanced Settings",
                        if account.teams.is_empty() {
                            "Refresh developer resources and account settings."
                        } else {
                            "Select the developer team for signing."
                        },
                    )
                    .on_click(cx.listener(Self::open_team_settings)),
                )
        } else {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(add_apple_account_button(cx))
        };

        Self::picker_shell(
            Self::section_header("1", "Account", "Apple Account and developer membership"),
            body,
            self.is_busy(),
        )
    }

    fn app_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let app = self.selected_app();
        let patch_summary = self.enabled_patch_names();
        let patch_summary = if patch_summary.is_empty() {
            "No patches enabled".to_string()
        } else {
            patch_summary.join(", ")
        };

        let body = div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.ipa_button(app, cx))
            .when_some(self.app_selection.load_error(), |this, error| {
                this.child(
                    div()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0xe5b8b2))
                        .bg(rgb(0xfff5f3))
                        .p_3()
                        .text_sm()
                        .text_color(rgb(0x9a302b))
                        .child(error.to_string()),
                )
            });

        let body = if let Some(app) = app {
            body.child(properties_list(vec![
                ("Bundle ID", app.bundle_id().to_string()),
                ("Version", format!("{} ({})", app.version(), app.build())),
                ("IPA", app.path.to_string()),
                ("Patches", patch_summary),
            ]))
            .child(
                Self::settings_button(
                    "app-disclosure",
                    "Advanced Settings",
                    "Edit app metadata and entitlements.",
                )
                .on_click(cx.listener(Self::open_app_settings)),
            )
        } else {
            body
        };

        Self::picker_shell(
            Self::section_header("2", "App", "IPA selection and patch profile"),
            body,
            self.is_busy(),
        )
    }

    fn ipa_button(&self, app: Option<&AppOption>, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.is_busy();
        let (label, title, detail) = if let Some(path) = self.app_selection.loading_path() {
            (
                path.to_string(),
                "Loading IPA...".into(),
                "Reading metadata, icon, and entitlements.".into(),
            )
        } else if let Some(app) = app {
            (app.path.clone(), app.name().clone(), app_identity(app))
        } else if let Some(path) = self.app_selection.error_path() {
            (
                path.to_string(),
                "Failed to load IPA".into(),
                "Select another IPA archive.".into(),
            )
        } else {
            (
                "No IPA selected".into(),
                "Choose IPA...".into(),
                "Select or drop an IPA archive.".into(),
            )
        };

        surface_button("ipa-button")
            .h(px(COMBO_ITEM_HEIGHT))
            .w_full()
            .can_drop(|dragged, _, _| {
                dragged
                    .downcast_ref::<ExternalPaths>()
                    .is_some_and(paths_include_ipa)
            })
            .drag_over::<ExternalPaths>(|this, paths, _, _| {
                if paths_include_ipa(paths) {
                    this.border_color(rgb(0x0f6f7a)).bg(rgb(0xf0fbfa))
                } else {
                    this
                }
            })
            .when(!disabled, |this| {
                this.cursor_pointer()
                    .on_click(cx.listener(Self::choose_ipa))
                    .on_drop(cx.listener(Self::drop_ipa))
            })
            .child(
                div()
                    .min_w_0()
                    .size_full()
                    .px_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xcfd8d6))
                    .bg(rgb(0xffffff))
                    .hover(|style| style.bg(rgb(0xeef6f5)))
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(select_item_content(label, title, detail))
                    .child(div().flex_none().w_7().h_7().child(icon_button_surface(
                        "icons/folder-open.svg",
                        0xebf1f0,
                        0xdfe8e6,
                        0x53666d,
                    ))),
            )
    }

    fn device_refresh_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        surface_button("refresh-devices")
            .w_7()
            .h_7()
            .when(
                !self.is_busy() && !self.device_selection.is_refreshing(),
                |this| {
                    this.cursor_pointer()
                        .on_click(cx.listener(Self::refresh_devices_from_button))
                },
            )
            .when(
                self.is_busy() || self.device_selection.is_refreshing(),
                |this| this.opacity(0.55).tab_stop(false),
            )
            .child(if self.device_selection.is_refreshing() {
                square_button_surface(
                    0xebf1f0,
                    0xdfe8e6,
                    0x53666d,
                    div()
                        .w_4()
                        .h_4()
                        .child(progress_circle(0.34, self.spinner_turns)),
                )
            } else {
                icon_button_surface("icons/refresh-cw.svg", 0xebf1f0, 0xdfe8e6, 0x53666d)
            })
    }

    fn device_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_open = self.open_picker == Some(PickerId::Device);
        let header = Self::section_header_with_action(
            "3",
            "Device",
            "Install target",
            self.device_refresh_button(cx),
        );

        let body = if let Some(device) = self.selected_device() {
            let trigger = select_button(
                "device-select",
                device.udid.clone(),
                device.name.clone(),
                device_identity(device),
                is_open,
            )
            .when(self.device_selection.is_refreshing(), |this| {
                this.tab_stop(false)
            });

            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(if self.device_selection.is_refreshing() {
                    trigger.into_any_element()
                } else {
                    select_with_popover(
                        "device-popover-scroll",
                        trigger,
                        is_open,
                        cx.listener(|this, open: &bool, _, cx| {
                            if this.is_busy() || this.device_selection.is_refreshing() {
                                this.open_picker = None;
                            } else {
                                this.open_picker = (*open).then_some(PickerId::Device);
                            }
                            cx.notify();
                        }),
                        dropdown_list(
                            (0..self.device_selection.len())
                                .map(|index| self.device_row(index, cx)),
                        )
                        .id("device-options"),
                    )
                    .into_any_element()
                })
                .child(properties_list(vec![
                    ("Model", device.model.to_string()),
                    ("OS", device.os.to_string()),
                    ("UDID", device.udid.to_string()),
                    ("Connection", device.connection.to_string()),
                ]))
        } else {
            let status = if self.device_selection.is_refreshing() {
                "Scanning for devices".to_string()
            } else if let Some(error) = self.device_selection.refresh_error() {
                error.to_string()
            } else {
                "No devices found".to_string()
            };
            let detail = if self.device_selection.is_refreshing() {
                "Looking for connected iPhone, iPad, Apple TV, or Apple Watch.".to_string()
            } else {
                "Connect a device, then refresh.".to_string()
            };

            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .h(px(COMBO_ITEM_HEIGHT))
                        .px_3()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0xcfd8d6))
                        .bg(rgb(0xffffff))
                        .flex()
                        .items_center()
                        .child(select_item_content(
                            "No UDID",
                            "No device connected",
                            detail,
                        )),
                )
                .child(properties_list(vec![("Status", status)]))
        };

        Self::picker_shell(header, body, self.is_busy())
    }

    fn plus_connector(disabled: bool) -> impl IntoElement {
        div()
            .w(px(PLUS_CONNECTOR_WIDTH))
            .min_h(px(340.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when(disabled, |this| this.opacity(0.55))
            .child(
                div()
                    .w_8()
                    .h_8()
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(0xcfd8d6))
                    .bg(rgb(0xfbfcfb))
                    .text_color(rgb(0x53666d))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(lucide_icon("icons/plus.svg")),
            )
    }

    fn sideload_connector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.sideload_operation.is_busy();
        let account_selected = self.selected_account().is_some();
        let selected_app = self.selected_app();
        let can_sign = account_selected && selected_app.is_some();
        let can_sideload = can_sign && self.selected_device().is_some();
        let progress = self.sideload_operation.progress();
        let track_fill_width = SIDELOAD_BUTTON_WIDTH * progress;
        let status_color = sideload_status_color(&self.sideload_operation);
        let status_text = sideload_status_text(
            &self.sideload_operation,
            account_selected,
            selected_app,
            self.selected_device(),
        );
        let status_label = if matches!(self.sideload_operation, SideloadOperation::Failed { .. }) {
            TextView::markdown("sideload-error-text", escape_markdown_text(&status_text))
                .selectable(true)
                .w(px(SIDELOAD_BUTTON_WIDTH))
                .text_center()
                .text_xs()
                .line_height(px(16.))
                .text_color(rgb(status_color))
                .into_any_element()
        } else {
            div()
                .w(px(SIDELOAD_BUTTON_WIDTH))
                .text_center()
                .text_xs()
                .line_height(px(16.))
                .text_color(rgb(status_color))
                .whitespace_normal()
                .child(status_text)
                .into_any_element()
        };

        div()
            .w(px(SIDELOAD_CONNECTOR_WIDTH))
            .min_h(px(340.))
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_1()
            .child(div().flex_none().w_full().h(px(SIDELOAD_STATUS_HEIGHT)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .w(px(18.))
                            .h(px(2.))
                            .rounded_full()
                            .bg(rgb(status_color)),
                    )
                    .child(self.sideload_dropdown_button(
                        busy,
                        can_sideload,
                        can_sign,
                        progress,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_none()
                            .w_6()
                            .h_9()
                            .text_color(rgb(status_color))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(connector_arrow_icon(status_color)),
                    ),
            )
            .child(
                div()
                    .min_h(px(SIDELOAD_STATUS_HEIGHT))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_none().w(px(18.)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .w(px(SIDELOAD_BUTTON_WIDTH))
                                    .h(px(3.))
                                    .rounded_full()
                                    .bg(rgb(0xcfd8d6))
                                    .child(
                                        div()
                                            .w(px(track_fill_width))
                                            .h_full()
                                            .rounded_full()
                                            .bg(rgb(status_color)),
                                    ),
                            )
                            .child(status_label),
                    )
                    .child(div().flex_none().w_6()),
            )
    }

    fn sideload_dropdown_button(
        &self,
        busy: bool,
        can_sideload: bool,
        can_sign: bool,
        progress: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.open_picker == Some(PickerId::SideloadAction);
        let menu_enabled = !busy && can_sign;
        let primary_enabled = !busy && can_sideload;
        let menu_width = 28.;
        let primary_width = SIDELOAD_BUTTON_WIDTH - menu_width;
        let primary = surface_button("sideload-primary-action")
            .w(px(primary_width))
            .h_9()
            .when(primary_enabled, |this| {
                this.cursor_pointer().on_click(cx.listener(Self::sideload))
            })
            .when(!primary_enabled, |this| this.opacity(0.65).tab_stop(false))
            .child(
                div()
                    .size_full()
                    .px_2()
                    .rounded_l_md()
                    .bg(fixed_rgb(0x173f45))
                    .text_color(fixed_rgb(0xffffff))
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(primary_enabled, |this| {
                        this.hover(|style| style.bg(fixed_rgb(0x20545c)))
                    })
                    .when(busy, |this| {
                        this.child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .gap_1()
                                .child(
                                    div()
                                        .flex_none()
                                        .w_4()
                                        .h_4()
                                        .child(progress_circle(progress, self.spinner_turns)),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .text_center()
                                        .text_ellipsis()
                                        .child(sideload_button_label(&self.sideload_operation)),
                                ),
                        )
                    })
                    .when(!busy, |this| {
                        this.child(sideload_button_label(&self.sideload_operation))
                    }),
            );
        let menu_trigger = surface_button("sideload-menu-trigger")
            .w(px(menu_width))
            .h_9()
            .when(menu_enabled, |this| {
                this.cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.is_busy() {
                            this.open_picker = None;
                        } else {
                            this.open_picker = (this.open_picker != Some(PickerId::SideloadAction))
                                .then_some(PickerId::SideloadAction);
                        }
                        cx.notify();
                    }))
            })
            .when(!menu_enabled, |this| this.opacity(0.65).tab_stop(false))
            .child(
                div()
                    .size_full()
                    .rounded_r_md()
                    .bg(fixed_rgb(0x14393f))
                    .text_color(fixed_rgb(0xffffff))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(menu_enabled, |this| {
                        this.hover(|style| style.bg(fixed_rgb(0x20545c)))
                    })
                    .child(lucide_icon_tinted("icons/chevron-down.svg", 0xffffff)),
            );
        let trigger = div()
            .id("sideload-action-button")
            .w(px(SIDELOAD_BUTTON_WIDTH))
            .h_9()
            .rounded_md()
            .overflow_hidden()
            .flex()
            .items_stretch()
            .when(!menu_enabled && !primary_enabled, |this| this.opacity(0.7))
            .child(primary)
            .child(menu_trigger);

        if !menu_enabled || busy {
            return trigger.into_any_element();
        }

        floating_menu_under(
            trigger,
            open,
            cx.listener(|this, open: &bool, _, cx| {
                if this.is_busy() {
                    this.open_picker = None;
                } else {
                    this.open_picker = (*open).then_some(PickerId::SideloadAction);
                }
                cx.notify();
            }),
            SIDELOAD_BUTTON_WIDTH,
            44.,
            self.sideload_action_menu(can_sign, cx),
        )
        .into_any_element()
    }

    fn sideload_action_menu(&self, can_sign: bool, cx: &mut Context<Self>) -> impl IntoElement {
        surface_button("sideload-menu-sign")
            .w_full()
            .h_8()
            .when(can_sign, |this| {
                this.cursor_pointer().on_click(cx.listener(Self::sign))
            })
            .when(!can_sign, |this| this.opacity(0.45).tab_stop(false))
            .child(
                div()
                    .size_full()
                    .px_3()
                    .rounded_sm()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0x173f45))
                    .flex()
                    .items_center()
                    .when(can_sign, |this| this.hover(|style| style.bg(rgb(0xeef6f5))))
                    .child("Sign"),
            )
    }
}

fn close_settings_window(handle: Option<SettingsWindowHandle>, cx: &mut App) {
    if let Some(handle) = handle {
        handle.close(cx);
    }
}

fn add_apple_account_button(cx: &mut Context<SideloaderView>) -> impl IntoElement {
    surface_button("add-apple-account")
        .h(px(COMBO_ITEM_HEIGHT))
        .w_full()
        .cursor_pointer()
        .on_click(cx.listener(SideloaderView::manage_accounts))
        .child(
            div()
                .min_w_0()
                .size_full()
                .px_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xcfd8d6))
                .bg(rgb(0xffffff))
                .hover(|style| style.bg(rgb(0xeef6f5)))
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(0x24333a))
                                .child("Add an Apple Account"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x7b8a90))
                                .child("Log in before choosing a developer team."),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .w_7()
                        .h_7()
                        .rounded_md()
                        .bg(rgb(0xebf1f0))
                        .text_color(rgb(0x53666d))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(lucide_icon("icons/plus.svg")),
                ),
        )
}

fn developer_team_count(account: &AccountOption) -> String {
    match account.teams.len() {
        0 => "No developer teams".to_string(),
        1 => "1 developer team".to_string(),
        count => format!("{count} developer teams"),
    }
}

fn sideload_top_label(operation: &SideloadOperation) -> &'static str {
    match operation {
        SideloadOperation::Idle => "Draft",
        SideloadOperation::Running { phase, .. } => sideload_phase_label(*phase),
        SideloadOperation::Finished { .. } => "Done",
        SideloadOperation::Failed { .. } => "Failed",
    }
}

fn sideload_status_color(operation: &SideloadOperation) -> u32 {
    match operation {
        SideloadOperation::Idle => 0x53666d,
        SideloadOperation::Running { .. } => 0x173f45,
        SideloadOperation::Finished { .. } => 0x1d6b45,
        SideloadOperation::Failed { .. } => 0x9a302b,
    }
}

fn sideload_button_label(operation: &SideloadOperation) -> &'static str {
    match operation {
        SideloadOperation::Idle
        | SideloadOperation::Finished { .. }
        | SideloadOperation::Failed { .. } => "Sideload",
        SideloadOperation::Running { phase, .. } => sideload_phase_label(*phase),
    }
}

fn sideload_status_text(
    operation: &SideloadOperation,
    account_selected: bool,
    app: Option<&AppOption>,
    device: Option<&DeviceOption>,
) -> String {
    match operation {
        SideloadOperation::Idle => {
            if !account_selected {
                "Choose an account".to_string()
            } else if app.is_none() {
                "Choose an IPA".to_string()
            } else if device.is_some() {
                "Ready to install".to_string()
            } else {
                "Ready to sign".to_string()
            }
        }
        SideloadOperation::Running {
            phase,
            progress,
            detail,
        } => {
            if !detail.is_empty() {
                format!("{detail} - {:.0}%", progress.clamp(0., 1.) * 100.)
            } else {
                match phase {
                    SideloadPhase::Signing => {
                        let app_name = app.map(|app| app.name().as_str()).unwrap_or("app");
                        format!("Signing {app_name}")
                    }
                    SideloadPhase::Installing => {
                        let device_name = device
                            .map(|device| device.name.as_str())
                            .unwrap_or("device");
                        format!("Installing to {device_name}")
                    }
                }
            }
        }
        SideloadOperation::Finished { message } => message.clone(),
        SideloadOperation::Failed { message } => message.clone(),
    }
}

fn escape_markdown_text(text: &str) -> String {
    text.chars()
        .flat_map(|character| {
            if matches!(
                character,
                '\\' | '`'
                    | '*'
                    | '_'
                    | '{'
                    | '}'
                    | '['
                    | ']'
                    | '<'
                    | '>'
                    | '#'
                    | '+'
                    | '-'
                    | '.'
                    | '!'
                    | '|'
            ) {
                vec!['\\', character]
            } else {
                vec![character]
            }
        })
        .collect()
}

fn sideload_phase_label(phase: SideloadPhase) -> &'static str {
    match phase {
        SideloadPhase::Signing => "Signing",
        SideloadPhase::Installing => "Installing",
    }
}

fn first_ipa_path(paths: &ExternalPaths) -> Option<PathBuf> {
    paths
        .paths()
        .iter()
        .find(|path| app_effects::is_ipa_path(path))
        .cloned()
}

fn signed_ipa_destination(app_path: &str) -> (PathBuf, String) {
    let path = Path::new(app_path);
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("app");
    (directory, format!("{stem}-signed.ipa"))
}

fn app_id_provisioning_prompt(plan: &app_effects::AppIdProvisioningPlan) -> (String, String) {
    let count = plan.app_ids.len();
    let message = format!("Create {count} App IDs before signing?");
    let mut detail = format!(
        "Super Sideloader needs to add these App IDs:\n\n{}",
        plan.app_ids
            .iter()
            .map(|app_id| format!("- {} ({})", app_id.name, app_id.identifier))
            .collect::<Vec<_>>()
            .join("\n")
    );
    if let Some(available) = plan.available_quantity {
        let remaining = plan.remaining_after_signing().unwrap_or_default();
        detail.push_str(&format!(
            "\n\nApple currently reports {available} App IDs remaining. {remaining} will remain after these are created."
        ));
    } else {
        detail.push_str("\n\nApple did not report the remaining App ID quota.");
    }
    (message, detail)
}

fn paths_include_ipa(paths: &ExternalPaths) -> bool {
    first_ipa_path(paths).is_some()
}

impl Render for SideloaderView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        sync_window_theme(window, cx, self.theme_preference);

        if self.sideload_operation.is_busy() || self.device_selection.is_refreshing() {
            self.spinner_turns = (self.spinner_turns + 0.035) % 1.;
            window.request_animation_frame();
        } else {
            self.spinner_turns = 0.;
        }

        let status_color = sideload_status_color(&self.sideload_operation);

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(rgb(0xf4f6f4))
            .text_color(rgb(0x263238))
            .font_family(".SystemUIFont")
            .can_drop(|dragged, _, _| {
                dragged
                    .downcast_ref::<ExternalPaths>()
                    .is_some_and(paths_include_ipa)
            })
            .on_drop(cx.listener(Self::drop_ipa))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_6()
                    .py_4()
                    .border_b_1()
                    .border_color(rgb(0xd7dfdc))
                    .bg(rgb(0xfbfcfb))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w_9()
                                    .h_9()
                                    .rounded_md()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(fixed_rgb(0x173f45))
                                    .text_color(fixed_rgb(0xffffff))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("S"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_weight(FontWeight::BOLD)
                                            .child("Super Sideloader"),
                                    )
                                    .child(
                                        div().text_xs().text_color(rgb(0x66767c)).child(
                                            "Choose account, app, and device before signing",
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().w_2().h_2().rounded_full().bg(rgb(status_color)))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(status_color))
                                            .child(sideload_top_label(&self.sideload_operation)),
                                    ),
                            )
                            .child(
                                surface_button("adi-settings-button")
                                    .w_8()
                                    .h_8()
                                    .cursor_pointer()
                                    .on_click(cx.listener(Self::open_adi_settings))
                                    .child(icon_button_surface(
                                        "icons/settings.svg",
                                        0xebf1f0,
                                        0xdfe8e6,
                                        0x53666d,
                                    )),
                            ),
                    ),
            )
            .child(
                div().p_5().child(
                    div()
                        .flex()
                        .items_start()
                        .gap_2()
                        .child(self.account_picker(cx))
                        .child(Self::plus_connector(self.is_busy()))
                        .child(self.app_picker(cx))
                        .child(self.sideload_connector(cx))
                        .child(self.device_picker(cx)),
                ),
            )
    }
}
