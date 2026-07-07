use crate::constants::*;
use crate::data::{
    sample_accounts, sample_adi_backends, sample_apps, sample_devices, sample_machine_identity,
};
use crate::models::{
    AccountOption, AdiBackendOption, AppOption, DeviceOption, MachineIdentity, PickerId,
    SideloadOperation, SideloadPhase, TeamOption,
};
use crate::settings::{show_or_open_settings_window, SettingsWindow, SettingsWindowRequest};
use crate::widgets::{
    app_identity, combo_button, combo_item_content, combo_with_popover, connector_arrow_icon,
    developer_account_title, device_identity, dropdown_list, lucide_icon, menu_action_row,
    progress_circle, properties_list,
};
use gpui::{
    div, prelude::*, px, rgb, App, ClickEvent, Context, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled,
    Window, WindowHandle,
};

pub(crate) struct SideloaderView {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) accounts: Vec<AccountOption>,
    pub(crate) apps: Vec<AppOption>,
    pub(crate) devices: Vec<DeviceOption>,
    pub(crate) adi_backends: Vec<AdiBackendOption>,
    pub(crate) selected_account: usize,
    pub(crate) selected_team: usize,
    pub(crate) selected_app: usize,
    pub(crate) selected_device: usize,
    pub(crate) selected_adi_backend: usize,
    pub(crate) machine_identity: MachineIdentity,
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

        let accounts = sample_accounts();
        let apps = sample_apps();
        let devices = sample_devices();
        let adi_backends = sample_adi_backends();
        let machine_identity = sample_machine_identity();
        let enabled_patches = apps
            .first()
            .map(|app| vec![false; app.patches.len()])
            .unwrap_or_default();

        Self {
            focus_handle,
            accounts,
            apps,
            devices,
            adi_backends,
            selected_account: 0,
            selected_team: 0,
            selected_app: 0,
            selected_device: 0,
            selected_adi_backend: 0,
            machine_identity,
            open_picker: None,
            enabled_patches,
            sideload_operation: SideloadOperation::Idle,
            spinner_turns: 0.,
            team_settings_window: None,
            app_settings_window: None,
            adi_settings_window: None,
        }
    }

    fn close_child_windows(&mut self, cx: &mut App) {
        close_settings_window(self.team_settings_window.take(), cx);
        close_settings_window(self.app_settings_window.take(), cx);
        close_settings_window(self.adi_settings_window.take(), cx);
    }

    fn is_busy(&self) -> bool {
        self.sideload_operation.is_busy()
    }

    fn selected_account(&self) -> &AccountOption {
        &self.accounts[self.selected_account]
    }

    fn selected_team(&self) -> &TeamOption {
        &self.selected_account().teams[self.selected_team]
    }

    fn selected_app(&self) -> &AppOption {
        &self.apps[self.selected_app]
    }

    fn selected_device(&self) -> &DeviceOption {
        &self.devices[self.selected_device]
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
            self.open_picker = None;
            cx.notify();
        }
    }

    fn select_app(
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
        if index < self.apps.len() {
            self.selected_app = index;
            self.enabled_patches = vec![false; self.selected_app().patches.len()];
            self.open_picker = None;
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
        if index < self.devices.len() {
            self.selected_device = index;
            self.open_picker = None;
            cx.notify();
        }
    }

    fn manage_accounts(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        self.open_picker = None;
        cx.notify();
    }

    fn choose_ipa(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        self.open_picker = None;
        cx.notify();
    }

    fn sideload(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        self.open_picker = None;
        self.sideload_operation = SideloadOperation::Running {
            phase: SideloadPhase::Signing,
            progress: 0.34,
        };
        cx.notify();
    }

    fn toggle_account_picker(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_picker(PickerId::Account, window, cx);
    }

    fn toggle_app_picker(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_picker(PickerId::App, window, cx);
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
        self.open_picker = None;
        let parent = cx.weak_entity();
        let request = SettingsWindowRequest::Team {
            parent,
            teams: self.selected_account().teams.clone(),
            selected_team: self.selected_team,
            auto_app_id: true,
            selected_app_id: 0,
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

    fn open_app_settings(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if self.is_busy() {
            return;
        }
        self.open_picker = None;
        let parent = cx.weak_entity();
        let request = SettingsWindowRequest::AppSettings {
            _parent: parent,
            app: self.selected_app().clone(),
            enabled_patches: self.enabled_patches.clone(),
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
        let parent = cx.weak_entity();
        let request = SettingsWindowRequest::AdiSettings {
            parent,
            backends: self.adi_backends.clone(),
            selected_backend: self.selected_adi_backend,
            machine_identity: self.machine_identity.clone(),
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

    fn enabled_patch_names(&self) -> Vec<String> {
        self.selected_app()
            .patches
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
    ) -> impl IntoElement {
        div().flex().items_start().justify_between().gap_3().child(
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

    fn app_row(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let app = &self.apps[index];
        let selected = index == self.selected_app;
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
            .id(("app-row", index))
            .on_click(
                cx.listener(move |this, event, window, cx| {
                    this.select_app(index, event, window, cx)
                }),
            )
            .flex()
            .items_center()
            .child(combo_item_content(
                app.path.clone(),
                app.name.clone(),
                app_identity(app),
            ))
    }

    fn device_row(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let device = &self.devices[index];
        let selected = index == self.selected_device;
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
        let account = self.selected_account();
        let team = self.selected_team();
        let is_open = self.open_picker == Some(PickerId::Account);

        Self::picker_shell(
            Self::section_header("1", "Account", "Apple ID and developer membership"),
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(combo_with_popover(
                    "account-popover-scroll",
                    combo_button(
                        account.apple_id.clone(),
                        account.label.clone(),
                        developer_account_title(&team.role),
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
                            "Manage Accounts...",
                            "Add, remove, or refresh Apple ID sessions.",
                        )
                        .id("manage-accounts")
                        .on_click(cx.listener(Self::manage_accounts)),
                    )
                    .id("account-options"),
                ))
                .child(properties_list(vec![
                    ("Apple ID", account.apple_id.to_string()),
                    ("Status", account.status.to_string()),
                    ("Source", account.detail.to_string()),
                    ("Team", team.name.to_string()),
                ]))
                .child(
                    Self::settings_button(
                        "Advanced Settings",
                        "Select the developer team for signing.",
                    )
                    .id("team-disclosure")
                    .on_click(cx.listener(Self::open_team_settings)),
                ),
            self.is_busy(),
        )
    }

    fn app_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let app = self.selected_app();
        let is_open = self.open_picker == Some(PickerId::App);
        let patch_summary = self.enabled_patch_names();
        let patch_summary = if patch_summary.is_empty() {
            "No patches enabled".to_string()
        } else {
            patch_summary.join(", ")
        };

        Self::picker_shell(
            Self::section_header("2", "App", "IPA selection and patch profile"),
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(combo_with_popover(
                    "app-popover-scroll",
                    combo_button(
                        app.path.clone(),
                        app.name.clone(),
                        app_identity(app),
                        is_open,
                    )
                    .id("app-combobox")
                    .on_click(cx.listener(Self::toggle_app_picker)),
                    is_open,
                    dropdown_list((0..self.apps.len()).map(|index| self.app_row(index, cx)))
                        .child(
                            menu_action_row("Choose IPA...", "Browse for another IPA archive.")
                                .id("choose-ipa")
                                .on_click(cx.listener(Self::choose_ipa)),
                        )
                        .id("app-options"),
                ))
                .child(properties_list(vec![
                    ("Bundle ID", app.bundle_id.to_string()),
                    ("Version", format!("{} ({})", app.version, app.build)),
                    ("IPA", app.path.to_string()),
                    ("Patches", patch_summary),
                ]))
                .child(
                    Self::settings_button(
                        "Advanced Settings",
                        "Edit app metadata and entitlements.",
                    )
                    .id("app-disclosure")
                    .on_click(cx.listener(Self::open_app_settings)),
                ),
            self.is_busy(),
        )
    }

    fn device_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let device = self.selected_device();
        let is_open = self.open_picker == Some(PickerId::Device);

        Self::picker_shell(
            Self::section_header("3", "Device", "Install target"),
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
                    .on_click(cx.listener(Self::toggle_device_picker)),
                    is_open,
                    dropdown_list((0..self.devices.len()).map(|index| self.device_row(index, cx)))
                        .id("device-options"),
                ))
                .child(properties_list(vec![
                    ("Model", device.model.to_string()),
                    ("OS", device.os.to_string()),
                    ("UDID", device.udid.to_string()),
                    ("Connection", device.connection.to_string()),
                ])),
            self.is_busy(),
        )
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
        let progress = self.sideload_operation.progress();
        let track_fill_width = SIDELOAD_BUTTON_WIDTH * progress;
        let status_color = self.sideload_operation.status_color();
        let status_text = self
            .sideload_operation
            .status_text(self.selected_app(), self.selected_device());

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
                            .when(!busy, |this| {
                                this.cursor_pointer()
                                    .hover(|style| style.bg(rgb(0x20545c)))
                                    .on_click(cx.listener(Self::sideload))
                            })
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

impl Render for SideloaderView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.sideload_operation.is_busy() {
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
