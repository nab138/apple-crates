use crate::adi_services;
use crate::app_selection::{is_ipa_path, load_ipa, AppSelection};
use crate::constants::*;
use crate::data::{load_accounts, load_machine_identity};
use crate::device_discovery::{discover_devices, watch_device_changes};
use crate::device_selection::DeviceSelection;
use crate::models::{
    AccountOption, AdiBackendKind, AdiBackendOption, AppOption, DeviceOption, MachineIdentity,
    PickerId, SideloadOperation, SideloadPhase, TeamOption,
};
use crate::preferences::{
    load_preferences, save_preferences as save_preferences_to_disk, AdiPreferences,
    AppOverridePreferences, AppPreferences, DeveloperPreferences, MachineIdentityPreferences,
    SideloaderPreferences, StoredAdiBackendKind,
};
use crate::settings::{show_or_open_settings_window, SettingsWindow, SettingsWindowRequest};
use crate::widgets::{
    app_identity, combo_button, combo_item_content, combo_with_popover, connector_arrow_icon,
    developer_account_title, device_identity, dropdown_list, lucide_icon, menu_action_row,
    progress_circle, properties_list,
};
use futures::{channel::mpsc, StreamExt};
use gpui::{
    div, prelude::*, px, rgb, App, ClickEvent, Context, ExternalPaths, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, ParentElement, PathPromptOptions, Render,
    StatefulInteractiveElement, Styled, Window, WindowHandle,
};
use rand::RngExt;
use std::path::PathBuf;

pub(crate) struct SideloaderView {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) accounts: Vec<AccountOption>,
    pub(crate) app_selection: AppSelection,
    pub(crate) device_selection: DeviceSelection,
    pub(crate) adi_backends: Vec<AdiBackendOption>,
    pub(crate) selected_account: usize,
    pub(crate) selected_team: usize,
    pub(crate) auto_app_id: bool,
    pub(crate) selected_app_id: usize,
    pub(crate) selected_adi_backend: usize,
    pub(crate) machine_identity: MachineIdentity,
    pub(crate) android_device_identity: MachineIdentity,
    pub(crate) android_adi_identifier: String,
    pub(crate) open_picker: Option<PickerId>,
    pub(crate) enabled_patches: Vec<bool>,
    pub(crate) sideload_operation: SideloadOperation,
    pub(crate) spinner_turns: f32,
    pub(crate) team_settings_window: Option<WindowHandle<SettingsWindow>>,
    pub(crate) app_settings_window: Option<WindowHandle<SettingsWindow>>,
    pub(crate) adi_settings_window: Option<WindowHandle<SettingsWindow>>,
}
impl SideloaderView {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        cx.on_release(|view, cx| view.close_child_windows(cx))
            .detach();

        let preferences = load_preferences().unwrap_or_else(|error| {
            eprintln!("{error}");
            SideloaderPreferences::default()
        });
        let accounts = load_accounts();
        let app_selection = AppSelection::from_preferences(&preferences.app);
        let machine_identity = load_machine_identity();
        let mut android_device_uuid = preferences
            .adi
            .android_device
            .machine_id
            .clone()
            .or_else(|| preferences.adi.android_device_uuid.clone())
            .unwrap_or_default();
        let generated_android_device_uuid = ensure_android_device_uuid(&mut android_device_uuid);
        let mut android_device_identity =
            android_device_identity_from_host(&machine_identity, android_device_uuid);
        apply_machine_identity_preferences(
            &mut android_device_identity,
            &preferences.adi.android_device,
        );
        let mut android_adi_identifier = preferences
            .adi
            .android_adi_identifier
            .clone()
            .or_else(|| {
                preferences
                    .adi
                    .android_machine
                    .machine_id
                    .clone()
                    .filter(|identifier| identifier.len() == 16)
            })
            .unwrap_or_default();
        let generated_android_adi_identifier =
            ensure_android_adi_identifier(&mut android_adi_identifier);
        let device_selection = DeviceSelection::new(preferences.device.as_ref());
        let adi_backends = adi_services::available_backends(&android_adi_identifier);
        let selected_account = selected_account_index(&accounts, &preferences.developer);
        let selected_team =
            selected_team_index(&accounts, selected_account, &preferences.developer);
        let selected_app_id = selected_app_id_index(
            &accounts,
            selected_account,
            selected_team,
            &preferences.developer,
        );
        let selected_adi_backend =
            selected_adi_backend_index(&adi_backends, preferences.adi.backend);
        let enabled_patches = app_selection
            .selected()
            .map(|app| vec![false; app.patches.len()])
            .unwrap_or_default();

        let mut view = Self {
            focus_handle,
            accounts,
            app_selection,
            device_selection,
            adi_backends,
            selected_account,
            selected_team,
            auto_app_id: preferences.developer.auto_app_id,
            selected_app_id,
            selected_adi_backend,
            machine_identity,
            android_device_identity,
            android_adi_identifier,
            open_picker: None,
            enabled_patches,
            sideload_operation: SideloadOperation::Idle,
            spinner_turns: 0.,
            team_settings_window: None,
            app_settings_window: None,
            adi_settings_window: None,
        };
        if generated_android_adi_identifier || generated_android_device_uuid {
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

    fn is_busy(&self) -> bool {
        self.sideload_operation.is_busy() || self.app_selection.is_loading()
    }

    pub(crate) fn save_preferences(&self) {
        if let Err(error) = save_preferences_to_disk(&self.preferences()) {
            eprintln!("{error}");
        }
    }

    pub(crate) fn select_team_from_settings(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Option<gpui::SharedString> {
        let team_count = self.accounts.get(self.selected_account)?.teams.len();
        if index >= team_count {
            return None;
        }

        self.selected_team = index;
        self.selected_app_id = 0;
        self.save_preferences();
        cx.notify();
        self.selected_team().map(|team| team.identifier.clone())
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

    pub(crate) fn select_app_id_from_settings(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected_app_id = index;
        self.save_preferences();
        cx.notify();
    }

    pub(crate) fn add_developer_account_from_settings(
        &mut self,
        account: AccountOption,
        cx: &mut Context<Self>,
    ) -> Option<SettingsWindowRequest> {
        self.accounts.push(account);
        self.selected_account = self.accounts.len().saturating_sub(1);
        self.selected_team = 0;
        self.selected_app_id = 0;
        self.open_picker = None;
        cx.notify();

        let account = self.accounts.get(self.selected_account)?;
        Some(SettingsWindowRequest::Team {
            parent: cx.weak_entity(),
            teams: account.teams.clone(),
            selected_team: self.selected_team,
            auto_app_id: self.auto_app_id,
            selected_app_id: self.selected_app_id,
        })
    }

    pub(crate) fn select_adi_backend_from_settings(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        if index < self.adi_backends.len() {
            self.selected_adi_backend = index;
            self.save_preferences();
            cx.notify();
        }
    }

    pub(crate) fn replace_app_from_settings(
        &mut self,
        app_index: usize,
        app: AppOption,
        cx: &mut Context<Self>,
    ) {
        if self.app_selection.replace(app_index, app) {
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
        self.adi_backends = backends;
        self.selected_adi_backend = selected_backend;
        if persist_selection {
            self.save_preferences();
        }
        cx.notify();
    }

    pub(crate) fn replace_android_device_identity_from_settings(
        &mut self,
        identity: MachineIdentity,
        cx: &mut Context<Self>,
    ) {
        self.android_device_identity = identity;
        self.save_preferences();
        cx.notify();
    }

    fn preferences(&self) -> SideloaderPreferences {
        let account = self.accounts.get(self.selected_account);
        let team = account.and_then(|account| account.teams.get(self.selected_team));
        let app_id = team.and_then(|team| team.app_ids.get(self.selected_app_id));
        let app = self.selected_app();
        let backend = self.adi_backends.get(self.selected_adi_backend);

        SideloaderPreferences {
            developer: DeveloperPreferences {
                account_id: account.map(|account| account.id.to_string()),
                team_id: team.map(|team| team.identifier.to_string()),
                auto_app_id: self.auto_app_id,
                app_id: app_id.map(|app_id| app_id.identifier.to_string()),
            },
            app: AppPreferences {
                bundle_id: app.map(|app| app.bundle_id().to_string()),
                path: self.app_selection.selected_path_for_preferences(),
                overrides: app.map(AppOverridePreferences::from).unwrap_or_default(),
            },
            device: self.device_selection.selected_preferences(),
            adi: AdiPreferences {
                backend: backend.map(|backend| StoredAdiBackendKind::from(backend.kind)),
                machine: MachineIdentityPreferences::from(&self.machine_identity),
                android_adi_identifier: Some(self.android_adi_identifier.clone()),
                android_device: MachineIdentityPreferences::from(&self.android_device_identity),
                android_device_uuid: Some(self.android_device_identity.machine_id.to_string()),
                android_machine: MachineIdentityPreferences::default(),
            },
        }
    }

    fn selected_account(&self) -> Option<&AccountOption> {
        self.accounts.get(self.selected_account)
    }

    fn selected_team(&self) -> Option<&TeamOption> {
        self.selected_account()
            .and_then(|account| account.teams.get(self.selected_team))
    }

    fn selected_app(&self) -> Option<&AppOption> {
        self.app_selection.selected()
    }

    fn selected_device(&self) -> Option<&DeviceOption> {
        self.device_selection.selected()
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
            self.selected_app_id = 0;
            self.open_picker = None;
            self.save_preferences();
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
        let request = SettingsWindowRequest::DeveloperLogin {
            parent: cx.weak_entity(),
        };
        self.team_settings_window = Some(show_or_open_settings_window(
            self.team_settings_window,
            request,
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
                    eprintln!("Failed to select IPA: {error}");
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
        if !is_ipa_path(&path) {
            let path = path.to_string_lossy().to_string();
            self.app_selection.fail(
                path.clone(),
                format!("Selected file is not an IPA archive: {path}"),
            );
            self.enabled_patches.clear();
            self.save_preferences();
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
                .background_spawn(async move { load_ipa(path, patches.clone()) })
                .await;

            let _ = view.update(cx, |view, cx| {
                view.app_selection.finish_loading(&path_for_error, result);
                view.enabled_patches = view
                    .selected_app()
                    .map(|app| vec![false; app.patches.len()])
                    .unwrap_or_default();
                view.save_preferences();
                cx.notify();
            });
        })
        .detach();
    }

    fn sideload(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() || self.selected_app().is_none() || self.selected_device().is_none() {
            return;
        }
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

    fn toggle_account_picker(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_picker(PickerId::Account, window, cx);
    }

    fn toggle_device_picker(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_picker(PickerId::Device, window, cx);
    }

    fn toggle_picker(&mut self, picker: PickerId, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        self.open_picker = if self.open_picker == Some(picker) {
            None
        } else {
            Some(picker)
        };
        cx.notify();
    }

    fn open_team_settings(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        let Some(account) = self.selected_account() else {
            return;
        };
        if account.teams.is_empty() || self.selected_team().is_none() {
            return;
        }
        let teams = account.teams.clone();
        self.open_picker = None;
        let parent = cx.weak_entity();
        let request = SettingsWindowRequest::Team {
            parent,
            teams,
            selected_team: self.selected_team,
            auto_app_id: self.auto_app_id,
            selected_app_id: self.selected_app_id,
        };
        self.team_settings_window = Some(show_or_open_settings_window(
            self.team_settings_window,
            request,
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
            let result = cx.background_spawn(async move { discover_devices() }).await;
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
        cx.background_spawn(async move { watch_device_changes(sender) })
            .detach();

        cx.spawn(async move |view, cx| {
            while let Some(result) = receiver.next().await {
                let should_continue = view
                    .update(cx, |view, cx| match result {
                        Ok(()) => {
                            view.device_selection.note_device_event();
                            if !view.is_busy() && !view.device_selection.is_refreshing() {
                                view.start_device_refresh(cx);
                            }
                            true
                        }
                        Err(error) => {
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
        let Some(app) = self.selected_app().cloned() else {
            return;
        };
        self.open_picker = None;
        let parent = cx.weak_entity();
        let request = SettingsWindowRequest::AppSettings {
            parent,
            app_index: self.app_selection.selected_index(),
            app,
            enabled_patches: self.enabled_patches.clone(),
            team_id: self
                .selected_team()
                .map(|team| team.identifier.clone())
                .unwrap_or_default(),
        };
        self.app_settings_window = Some(show_or_open_settings_window(
            self.app_settings_window,
            request,
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
        let parent = cx.weak_entity();
        let request = SettingsWindowRequest::AdiSettings {
            parent,
            backends: self.adi_backends.clone(),
            selected_backend: self.selected_adi_backend,
            machine_identity: self.machine_identity.clone(),
            android_device_identity: self.android_device_identity.clone(),
            android_adi_identifier: self.android_adi_identifier.clone(),
        };
        self.adi_settings_window = Some(show_or_open_settings_window(
            self.adi_settings_window,
            request,
            ADI_SETTINGS_WINDOW_WIDTH,
            ADI_SETTINGS_WINDOW_HEIGHT,
            cx,
        ));
        cx.notify();
    }

    pub(crate) fn refresh_adi_backends(&mut self) {
        let selected_kind = self
            .adi_backends
            .get(self.selected_adi_backend)
            .map(|backend| backend.kind);
        self.adi_backends = adi_services::available_backends(&self.android_adi_identifier);
        self.selected_adi_backend = selected_kind
            .and_then(|kind| {
                self.adi_backends
                    .iter()
                    .position(|backend| backend.kind == kind)
            })
            .unwrap_or_else(|| adi_services::default_backend(&self.adi_backends));
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
                            .bg(rgb(0x173f45))
                            .text_color(rgb(0xffffff))
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

    fn settings_button(label: &'static str, detail: &'static str) -> gpui::Div {
        div()
            .min_h(px(40.))
            .px_3()
            .py_1()
            .rounded_md()
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .hover(|style| style.bg(rgb(0xf4f7f7)))
            .child(
                div()
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
            .child(
                div()
                    .w_6()
                    .h_6()
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgb(0xebf1f0))
                    .text_color(rgb(0x53666d))
                    .child(lucide_icon("icons/wrench.svg")),
            )
    }

    fn account_row(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let account = &self.accounts[index];
        let selected = index == self.selected_account;
        let detail = account
            .teams
            .first()
            .map(|team| developer_account_title(&team.role))
            .unwrap_or_else(|| account.status.clone());
        let border = if selected {
            rgb(0x0f6f7a)
        } else {
            rgb(0xd8e0df)
        };
        let bg = if selected {
            rgb(0xf0fbfa)
        } else {
            rgb(0xffffff)
        };

        div()
            .h(px(COMBO_ITEM_HEIGHT))
            .px_3()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(bg)
            .cursor_pointer()
            .hover(|style| style.bg(rgb(0xf6f9f9)))
            .id(("account-row", index))
            .on_click(cx.listener(move |this, event, window, cx| {
                this.select_account(index, event, window, cx)
            }))
            .flex()
            .items_center()
            .child(combo_item_content(
                account.apple_id.clone(),
                account.label.clone(),
                detail,
            ))
    }

    fn device_row(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let device = self
            .device_selection
            .device(index)
            .expect("device row index must be valid");
        let selected = index == self.device_selection.selected_index();
        let border = if selected {
            rgb(0x0f6f7a)
        } else {
            rgb(0xd8e0df)
        };

        div()
            .h(px(COMBO_ITEM_HEIGHT))
            .px_3()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(if selected {
                rgb(0xf0fbfa)
            } else {
                rgb(0xffffff)
            })
            .cursor_pointer()
            .hover(|style| style.bg(rgb(0xf6f9f9)))
            .id(("device-row", index))
            .on_click(cx.listener(move |this, event, window, cx| {
                this.select_device(index, event, window, cx)
            }))
            .flex()
            .items_center()
            .child(combo_item_content(
                device.udid.clone(),
                device.name.clone(),
                device_identity(device),
            ))
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
            let detail = team
                .map(|team| developer_account_title(&team.role))
                .unwrap_or_else(|| account.status.clone());
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
                .child(combo_with_popover(
                    "account-popover-scroll",
                    combo_button(
                        account.apple_id.clone(),
                        account.label.clone(),
                        detail,
                        is_open,
                    )
                    .id("account-combobox")
                    .on_click(cx.listener(Self::toggle_account_picker)),
                    is_open,
                    dropdown_list(
                        (0..self.accounts.len()).map(|index| self.account_row(index, cx)),
                    )
                    .child(
                        menu_action_row(
                            "Add an Apple Account",
                            "Log in to another developer Apple Account.",
                        )
                        .id("manage-accounts")
                        .on_click(cx.listener(Self::manage_accounts)),
                    )
                    .id("account-options"),
                ))
                .child(properties_list(properties))
                .when(team.is_some(), |this| {
                    this.child(
                        Self::settings_button(
                            "Advanced Settings",
                            "Select the developer team for signing.",
                        )
                        .id("team-disclosure")
                        .on_click(cx.listener(Self::open_team_settings)),
                    )
                })
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
                Self::settings_button("Advanced Settings", "Edit app metadata and entitlements.")
                    .id("app-disclosure")
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
                path.to_string().into(),
                "Loading IPA...".into(),
                "Reading metadata, icon, and entitlements.".into(),
            )
        } else if let Some(app) = app {
            (app.path.clone(), app.name().clone(), app_identity(app))
        } else if let Some(path) = self.app_selection.error_path() {
            (
                path.to_string().into(),
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

        div()
            .id("ipa-button")
            .h(px(COMBO_ITEM_HEIGHT))
            .px_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xcfd8d6))
            .bg(rgb(0xffffff))
            .flex()
            .items_center()
            .gap_3()
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
                    .hover(|style| style.bg(rgb(0xf6f9f9)))
                    .on_click(cx.listener(Self::choose_ipa))
                    .on_drop(cx.listener(Self::drop_ipa))
            })
            .child(combo_item_content(label, title, detail))
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
                    .child(lucide_icon("icons/folder-open.svg")),
            )
    }

    fn device_refresh_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("refresh-devices")
            .w_7()
            .h_7()
            .rounded_md()
            .bg(rgb(0xebf1f0))
            .text_color(rgb(0x53666d))
            .flex()
            .items_center()
            .justify_center()
            .when(
                !self.is_busy() && !self.device_selection.is_refreshing(),
                |this| {
                    this.cursor_pointer()
                        .hover(|style| style.bg(rgb(0xdfe8e6)))
                        .on_click(cx.listener(Self::refresh_devices_from_button))
                },
            )
            .when(self.is_busy(), |this| this.opacity(0.55))
            .child(if self.device_selection.is_refreshing() {
                div()
                    .w_4()
                    .h_4()
                    .child(progress_circle(0.34, self.spinner_turns))
            } else {
                div().child(lucide_icon("icons/refresh-cw.svg"))
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
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(combo_with_popover(
                    "device-popover-scroll",
                    combo_button(
                        device.udid.clone(),
                        device.name.clone(),
                        device_identity(device),
                        is_open,
                    )
                    .id("device-combobox")
                    .when(!self.device_selection.is_refreshing(), |this| {
                        this.on_click(cx.listener(Self::toggle_device_picker))
                    }),
                    is_open,
                    dropdown_list(
                        (0..self.device_selection.len()).map(|index| self.device_row(index, cx)),
                    )
                    .id("device-options"),
                ))
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
                        .child(combo_item_content("No UDID", "No device connected", detail)),
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
        let selected_app = self.selected_app();
        let can_sideload = selected_app.is_some() && self.selected_device().is_some();
        let progress = self.sideload_operation.progress();
        let track_fill_width = SIDELOAD_BUTTON_WIDTH * progress;
        let status_color = self.sideload_operation.status_color();
        let status_text = self
            .sideload_operation
            .status_text(selected_app, self.selected_device());

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
                    .child(
                        div()
                            .id("sideload-button")
                            .w(px(SIDELOAD_BUTTON_WIDTH))
                            .h_9()
                            .px_2()
                            .rounded_md()
                            .bg(rgb(0x173f45))
                            .text_color(rgb(0xffffff))
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap_1()
                            .when(!busy && can_sideload, |this| {
                                this.cursor_pointer()
                                    .hover(|style| style.bg(rgb(0x20545c)))
                                    .on_click(cx.listener(Self::sideload))
                            })
                            .when(!can_sideload, |this| this.opacity(0.7))
                            .when(busy, |this| {
                                this.child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div().flex_none().w_4().h_4().child(progress_circle(
                                                progress,
                                                self.spinner_turns,
                                            )),
                                        )
                                        .child(
                                            div()
                                                .min_w_0()
                                                .flex_1()
                                                .text_center()
                                                .text_ellipsis()
                                                .child(self.sideload_operation.button_label()),
                                        )
                                        .child(div().flex_none().w_4().h_4()),
                                )
                            })
                            .when(!busy, |this| {
                                this.child(self.sideload_operation.button_label())
                            }),
                    )
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
}

fn close_settings_window(handle: Option<WindowHandle<SettingsWindow>>, cx: &mut App) {
    if let Some(handle) = handle {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

fn add_apple_account_button(cx: &mut Context<SideloaderView>) -> impl IntoElement {
    div()
        .id("add-apple-account")
        .h(px(COMBO_ITEM_HEIGHT))
        .px_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xcfd8d6))
        .bg(rgb(0xffffff))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xf6f9f9)))
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .on_click(cx.listener(SideloaderView::manage_accounts))
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_1()
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
        )
}

fn selected_account_index(accounts: &[AccountOption], preferences: &DeveloperPreferences) -> usize {
    preferences
        .account_id
        .as_deref()
        .and_then(|account_id| {
            accounts
                .iter()
                .position(|account| account.id.as_ref() == account_id)
        })
        .unwrap_or(0)
}

fn selected_team_index(
    accounts: &[AccountOption],
    selected_account: usize,
    preferences: &DeveloperPreferences,
) -> usize {
    preferences
        .team_id
        .as_deref()
        .and_then(|team_id| {
            accounts
                .get(selected_account)?
                .teams
                .iter()
                .position(|team| team.identifier.as_ref() == team_id)
        })
        .unwrap_or(0)
}

fn selected_app_id_index(
    accounts: &[AccountOption],
    selected_account: usize,
    selected_team: usize,
    preferences: &DeveloperPreferences,
) -> usize {
    preferences
        .app_id
        .as_deref()
        .and_then(|app_id| {
            accounts
                .get(selected_account)?
                .teams
                .get(selected_team)?
                .app_ids
                .iter()
                .position(|candidate| candidate.identifier.as_ref() == app_id)
        })
        .unwrap_or(0)
}

fn selected_adi_backend_index(
    backends: &[AdiBackendOption],
    backend: Option<StoredAdiBackendKind>,
) -> usize {
    backend
        .and_then(|backend| {
            let backend = AdiBackendKind::from(backend);
            backends
                .iter()
                .position(|candidate| candidate.kind == backend)
        })
        .unwrap_or_else(|| adi_services::default_backend(backends))
}

fn ensure_android_adi_identifier(identifier: &mut String) -> bool {
    if identifier.len() == 16 {
        return false;
    }

    *identifier = random_android_adi_identifier();
    true
}

fn ensure_android_device_uuid(identifier: &mut String) -> bool {
    if is_uuid(identifier) {
        return false;
    }

    *identifier = uuid::Uuid::new_v4().hyphenated().to_string().to_uppercase();
    true
}

fn is_uuid(identifier: &str) -> bool {
    uuid::Uuid::parse_str(identifier).is_ok()
}

fn android_device_identity_from_host(
    host_identity: &MachineIdentity,
    device_uuid: String,
) -> MachineIdentity {
    MachineIdentity {
        machine_name: host_identity.machine_name.clone(),
        os_name: host_identity.os_name.clone(),
        os_version: host_identity.os_version.clone(),
        machine_id: device_uuid.into(),
    }
}

fn apply_machine_identity_preferences(
    identity: &mut MachineIdentity,
    preferences: &MachineIdentityPreferences,
) {
    if let Some(machine_name) = preferences.machine_name.as_ref() {
        identity.machine_name = machine_name.clone().into();
    }
    if let Some(os_name) = preferences.os_name.as_ref() {
        identity.os_name = os_name.clone().into();
    }
    if let Some(os_version) = preferences.os_version.as_ref() {
        identity.os_version = os_version.clone().into();
    }
    if let Some(machine_id) = preferences.machine_id.as_ref() {
        identity.machine_id = machine_id.clone().into();
    }
}

fn random_android_adi_identifier() -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut rng = rand::rng();
    let mut id = String::with_capacity(16);
    for _ in 0..16 {
        let index: usize = rng.random_range(0..HEX.len());
        id.push(HEX[index] as char);
    }
    id
}

fn first_ipa_path(paths: &ExternalPaths) -> Option<PathBuf> {
    paths.paths().iter().find(|path| is_ipa_path(path)).cloned()
}

fn paths_include_ipa(paths: &ExternalPaths) -> bool {
    first_ipa_path(paths).is_some()
}

impl Render for SideloaderView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.sideload_operation.is_busy() || self.device_selection.is_refreshing() {
            self.spinner_turns = (self.spinner_turns + 0.035) % 1.;
            window.request_animation_frame();
        } else {
            self.spinner_turns = 0.;
        }

        let status_color = self.sideload_operation.status_color();

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
                                    .bg(rgb(0x173f45))
                                    .text_color(rgb(0xffffff))
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
                                            .child(self.sideload_operation.top_label()),
                                    ),
                            )
                            .child(
                                div()
                                    .id("adi-settings-button")
                                    .w_8()
                                    .h_8()
                                    .rounded_md()
                                    .bg(rgb(0xebf1f0))
                                    .text_color(rgb(0x53666d))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|style| style.bg(rgb(0xdfe8e6)))
                                    .on_click(cx.listener(Self::open_adi_settings))
                                    .child(lucide_icon("icons/settings.svg")),
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
