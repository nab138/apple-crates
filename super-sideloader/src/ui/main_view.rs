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
    InteractiveElement, IntoElement, ParentElement, PathPromptOptions, Render, Styled, Window,
};
use gpui_component::button::Button;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

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

    fn sideload(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy()
            || self.selected_account().is_none()
            || self.selected_app().is_none()
            || self.selected_device().is_none()
        {
            return;
        }
        self.start_signing_operation(cx);
    }

    fn sign(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() || self.selected_account().is_none() || self.selected_app().is_none() {
            return;
        }
        self.start_signing_operation(cx);
    }

    fn start_signing_operation(&mut self, cx: &mut Context<Self>) {
        self.open_picker = None;
        self.sideload_operation = SideloadOperation::Running {
            phase: SideloadPhase::Signing,
            progress: 0.34,
        };
        cx.notify();
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
                    .h(px(SIDELOAD_STATUS_HEIGHT))
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
                            .child(
                                div()
                                    .w(px(SIDELOAD_BUTTON_WIDTH))
                                    .text_center()
                                    .text_xs()
                                    .text_color(rgb(status_color))
                                    .text_ellipsis()
                                    .child(status_text),
                            ),
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
        SideloadOperation::Finished => "Done",
        SideloadOperation::Failed { .. } => "Failed",
    }
}

fn sideload_status_color(operation: &SideloadOperation) -> u32 {
    match operation {
        SideloadOperation::Idle => 0x53666d,
        SideloadOperation::Running { .. } => 0x173f45,
        SideloadOperation::Finished => 0x1d6b45,
        SideloadOperation::Failed { .. } => 0x9a302b,
    }
}

fn sideload_button_label(operation: &SideloadOperation) -> &'static str {
    match operation {
        SideloadOperation::Idle
        | SideloadOperation::Finished
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
        SideloadOperation::Running { phase, .. } => match phase {
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
        },
        SideloadOperation::Finished => {
            let device_name = device
                .map(|device| device.name.as_str())
                .unwrap_or("device");
            format!("Installed on {device_name}")
        }
        SideloadOperation::Failed { message } => message.clone(),
    }
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
