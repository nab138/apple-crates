use crate::constants::*;
use crate::main_view::SideloaderView;
use crate::models::{AdiBackendOption, AppIdOption, AppOption, MachineIdentity, TeamOption};
use crate::widgets::{
    chevron, combo_button, combo_item_content, combo_with_popover, developer_account_title,
    dropdown_list, lucide_icon, properties_list,
};
use gpui::{
    anchored, deferred, div, point, prelude::*, px, rgb, size, AnchoredPositionMode, App,
    AppContext, Bounds, ClickEvent, Context, FocusHandle, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement,
    Styled, WeakEntity, Window, WindowBounds, WindowHandle, WindowKind, WindowOptions,
};

#[derive(Clone)]
pub(crate) enum SettingsWindowRequest {
    Team {
        parent: WeakEntity<SideloaderView>,
        teams: Vec<TeamOption>,
        selected_team: usize,
        auto_app_id: bool,
        selected_app_id: usize,
    },
    AppSettings {
        _parent: WeakEntity<SideloaderView>,
        app: AppOption,
        enabled_patches: Vec<bool>,
    },
    AdiSettings {
        parent: WeakEntity<SideloaderView>,
        backends: Vec<AdiBackendOption>,
        selected_backend: usize,
        machine_identity: MachineIdentity,
    },
}

pub(crate) struct SettingsWindow {
    focus_handle: FocusHandle,
    request: SettingsWindowRequest,
    scroll_handle: ScrollHandle,
    team_picker_open: bool,
    app_id_picker_open: bool,
    adi_backend_picker_open: bool,
}
impl SettingsWindowRequest {
    fn title(&self) -> &'static str {
        match self {
            SettingsWindowRequest::Team { .. } => "Developer Settings",
            SettingsWindowRequest::AppSettings { .. } => "App Settings",
            SettingsWindowRequest::AdiSettings { .. } => "Settings",
        }
    }
}

impl SettingsWindow {
    fn new(request: SettingsWindowRequest, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        cx.defer_in(window, |_, _, cx| cx.notify());

        Self {
            focus_handle,
            request,
            scroll_handle: ScrollHandle::new(),
            team_picker_open: false,
            app_id_picker_open: false,
            adi_backend_picker_open: false,
        }
    }

    fn show_request(
        &mut self,
        request: SettingsWindowRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request = request;
        self.team_picker_open = false;
        self.app_id_picker_open = false;
        self.adi_backend_picker_open = false;
        window.activate_window();
        window.focus(&self.focus_handle, cx);
        cx.defer_in(window, |_, _, cx| cx.notify());
        cx.notify();
    }

    fn select_team(
        &mut self,
        index: usize,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let SettingsWindowRequest::Team {
            parent,
            teams,
            selected_team,
            selected_app_id,
            ..
        } = &mut self.request
        else {
            return;
        };

        if index >= teams.len() {
            return;
        }

        *selected_team = index;
        *selected_app_id = 0;
        self.team_picker_open = false;
        self.app_id_picker_open = false;
        let _ = parent.update(cx, |view, cx| {
            let team_count = view.accounts[view.selected_account].teams.len();
            if index < team_count {
                view.selected_team = index;
                cx.notify();
            }
        });
        cx.notify();
    }

    fn toggle_team_picker(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        if matches!(self.request, SettingsWindowRequest::Team { .. }) {
            self.team_picker_open = !self.team_picker_open;
            self.app_id_picker_open = false;
            cx.notify();
        }
    }

    fn toggle_auto_app_id(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::Team { auto_app_id, .. } = &mut self.request else {
            return;
        };

        *auto_app_id = !*auto_app_id;
        self.app_id_picker_open = false;
        cx.notify();
    }

    fn toggle_app_id_picker(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::Team { auto_app_id, .. } = &self.request else {
            return;
        };

        if !*auto_app_id {
            self.app_id_picker_open = !self.app_id_picker_open;
            self.team_picker_open = false;
            cx.notify();
        }
    }

    fn select_app_id(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::Team {
            teams,
            selected_team,
            selected_app_id,
            ..
        } = &mut self.request
        else {
            return;
        };

        let Some(team) = teams.get(*selected_team) else {
            return;
        };

        if index < team.app_ids.len() {
            *selected_app_id = index;
            self.app_id_picker_open = false;
            cx.notify();
        }
    }

    fn add_app_id(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn remove_app_id(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn edit_app_id(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn refresh_team_details(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if matches!(self.request, SettingsWindowRequest::Team { .. }) {
            self.team_picker_open = false;
            self.app_id_picker_open = false;
            cx.notify();
        }
    }

    fn select_adi_backend(
        &mut self,
        index: usize,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let SettingsWindowRequest::AdiSettings {
            parent,
            backends,
            selected_backend,
            ..
        } = &mut self.request
        else {
            return;
        };

        if index >= backends.len() {
            return;
        }

        *selected_backend = index;
        self.adi_backend_picker_open = false;
        let _ = parent.update(cx, |view, cx| {
            if index < view.adi_backends.len() {
                view.selected_adi_backend = index;
                cx.notify();
            }
        });
        cx.notify();
    }

    fn toggle_adi_backend_picker(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if matches!(self.request, SettingsWindowRequest::AdiSettings { .. }) {
            self.adi_backend_picker_open = !self.adi_backend_picker_open;
            cx.notify();
        }
    }

    fn repair_adi_backend(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn edit_machine_identity(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn edit_app_icon(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn edit_app_detail(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn revert_app_detail(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn add_entitlement(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn remove_entitlement(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn revert_entitlements(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let team_picker_open = self.team_picker_open;
        let app_id_picker_open = self.app_id_picker_open;
        let adi_backend_picker_open = self.adi_backend_picker_open;

        match &mut self.request {
            SettingsWindowRequest::Team {
                teams,
                selected_team,
                auto_app_id,
                selected_app_id,
                ..
            } => {
                let selected = *selected_team;
                let auto_app_id = *auto_app_id;
                let selected_app_id = *selected_app_id;
                let team = teams.get(selected);
                let mut content = div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .when_some(team, |this, team| {
                        this.child(developer_settings_section(
                            "Developer Team",
                            "Team used for certificates, profiles, and signing identities.",
                            div()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(combo_with_popover(
                                    "team-popover-scroll",
                                    combo_button(
                                        team.identifier.clone(),
                                        team.name.clone(),
                                        developer_account_title(&team.role),
                                        team_picker_open,
                                    )
                                    .id("settings-team-combobox")
                                    .on_click(cx.listener(Self::toggle_team_picker)),
                                    team_picker_open,
                                    dropdown_list((0..teams.len()).map(|index| {
                                        settings_team_row(&teams[index], index == selected)
                                            .id(("settings-team-row", index))
                                            .on_click(cx.listener(
                                                move |this, event, window, cx| {
                                                    this.select_team(index, event, window, cx)
                                                },
                                            ))
                                    }))
                                    .id("settings-team-options"),
                                ))
                                .child(properties_list(vec![
                                    ("Name", team.name.to_string()),
                                    ("Team ID", team.identifier.to_string()),
                                    ("Role", team.role.to_string()),
                                ])),
                        ))
                    });

                if let Some(team) = team {
                    content = content.child(app_id_section(
                        team,
                        auto_app_id,
                        selected_app_id,
                        app_id_picker_open,
                        cx,
                    ));
                }

                settings_window_shell()
                    .track_focus(&self.focus_handle)
                    .gap_4()
                    .child(settings_window_header_with_action(
                        settings_window_header(
                            "Developer Settings",
                            "Configure developer-team resources used during signing.",
                        ),
                        team_refresh_button(cx),
                    ))
                    .child(scroll_panel(
                        "team-settings-scroll",
                        &self.scroll_handle,
                        content,
                    ))
            }
            SettingsWindowRequest::AppSettings {
                app,
                enabled_patches,
                ..
            } => {
                let patch_summary = enabled_patch_summary(app, enabled_patches);

                settings_window_shell()
                    .track_focus(&self.focus_handle)
                    .child(scroll_panel(
                        "app-settings-scroll",
                        &self.scroll_handle,
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(settings_window_header(
                                "App Settings",
                                "Review metadata and entitlements before signing.",
                            ))
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .p_3()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0xd8e0df))
                                    .bg(rgb(0xffffff))
                                    .child(app_icon_editor(app, cx))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex()
                                            .flex_1()
                                            .flex_col()
                                            .gap_2()
                                            .child(app_detail_row(
                                                "Name",
                                                app.name.to_string(),
                                                true,
                                                cx,
                                            ))
                                            .child(app_detail_row(
                                                "Bundle ID",
                                                app.bundle_id.to_string(),
                                                true,
                                                cx,
                                            ))
                                            .child(app_detail_row(
                                                "Version",
                                                app.version.to_string(),
                                                true,
                                                cx,
                                            ))
                                            .child(app_detail_row(
                                                "Build",
                                                app.build.to_string(),
                                                true,
                                                cx,
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .p_3()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0xd8e0df))
                                    .bg(rgb(0xffffff))
                                    .child(app_detail_row("IPA", app.path.to_string(), false, cx))
                                    .child(app_detail_row(
                                        "Executable",
                                        app.name.to_string().replace(' ', ""),
                                        true,
                                        cx,
                                    ))
                                    .child(app_detail_row(
                                        "Minimum OS",
                                        "iOS 15.0".to_string(),
                                        true,
                                        cx,
                                    ))
                                    .child(app_detail_row(
                                        "Supported devices",
                                        "iPhone, iPad".to_string(),
                                        true,
                                        cx,
                                    ))
                                    .child(app_detail_row("Patches", patch_summary, false, cx)),
                            )
                            .child(entitlements_table(app, cx)),
                    ))
            }
            SettingsWindowRequest::AdiSettings {
                backends,
                selected_backend,
                machine_identity,
                ..
            } => {
                let selected = *selected_backend;
                let backend = backends.get(selected);
                let editable_identity = backend
                    .map(|backend| backend.editable_identity)
                    .unwrap_or(false);

                let mut content = div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(settings_window_header(
                        "Settings",
                        "Choose the ADI backend and review the machine identity it uses.",
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0x66767c))
                                    .child("ADI Backend"),
                            )
                            .when_some(backend, |this, backend| {
                                this.child(settings_backend_combo_with_popover(
                                    settings_adi_backend_combo_button(
                                        backend,
                                        adi_backend_picker_open,
                                    )
                                    .id("adi-backend-combobox")
                                    .on_click(cx.listener(Self::toggle_adi_backend_picker)),
                                    adi_backend_picker_open,
                                    dropdown_list((0..backends.len()).map(|index| {
                                        settings_adi_backend_option(
                                            &backends[index],
                                            index == selected,
                                        )
                                        .id(("adi-backend-option", index))
                                        .on_click(
                                            cx.listener(move |this, event, window, cx| {
                                                this.select_adi_backend(index, event, window, cx)
                                            }),
                                        )
                                    }))
                                    .id("adi-backend-options"),
                                ))
                            }),
                    );

                if let Some(backend) = backend {
                    content = content.child(adi_backend_status_section(backend, cx));
                }

                content = content.child(machine_identity_section(
                    machine_identity,
                    editable_identity,
                    cx,
                ));

                settings_window_shell()
                    .track_focus(&self.focus_handle)
                    .child(scroll_panel(
                        "adi-settings-scroll",
                        &self.scroll_handle,
                        content,
                    ))
            }
        }
    }
}

pub(crate) fn show_or_open_settings_window(
    handle: Option<WindowHandle<SettingsWindow>>,
    request: SettingsWindowRequest,
    width: f32,
    height: f32,
    cx: &mut App,
) -> WindowHandle<SettingsWindow> {
    if let Some(handle) = handle {
        if handle
            .update(cx, |settings, window, cx| {
                settings.show_request(request.clone(), window, cx);
            })
            .is_ok()
        {
            return handle;
        }
    }

    open_settings_window(request, width, height, cx)
}

fn open_settings_window(
    request: SettingsWindowRequest,
    width: f32,
    height: f32,
    cx: &mut App,
) -> WindowHandle<SettingsWindow> {
    let window_size = size(px(width), px(height));
    let bounds = Bounds::centered(None, window_size, cx);
    let title = request.title();

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(window_size),
            is_resizable: true,
            is_minimizable: false,
            kind: WindowKind::Floating,
            ..Default::default()
        },
        |window, cx| {
            window.set_window_title(title);
            cx.new(|cx| SettingsWindow::new(request, window, cx))
        },
    )
    .expect("failed to open settings window")
}

fn scroll_panel(
    scroll_id: &'static str,
    scroll_handle: &ScrollHandle,
    content: impl IntoElement,
) -> gpui::Div {
    let viewport_height = scroll_handle.bounds().size.height.as_f32();
    let max_scroll = scroll_handle.max_offset().y.as_f32();
    let is_scrollable = viewport_height > 0. && max_scroll > 0.5;
    let scroll_offset = (-scroll_handle.offset().y.as_f32()).clamp(0., max_scroll);
    let content_height = viewport_height + max_scroll;
    let thumb_height = if is_scrollable {
        ((viewport_height / content_height) * viewport_height).clamp(36., viewport_height)
    } else {
        0.
    };
    let thumb_top = if is_scrollable && max_scroll > 0. {
        (scroll_offset / max_scroll) * (viewport_height - thumb_height)
    } else {
        0.
    };

    div()
        .min_h_0()
        .flex_1()
        .flex()
        .gap_2()
        .child(
            div()
                .id(scroll_id)
                .min_w_0()
                .h_full()
                .flex_1()
                .overflow_y_scroll()
                .scrollbar_width(px(1.))
                .track_scroll(scroll_handle)
                .child(content),
        )
        .when(is_scrollable, |this| {
            this.child(
                div()
                    .flex_none()
                    .w(px(6.))
                    .h_full()
                    .rounded_full()
                    .bg(rgb(0xdde5e4))
                    .flex()
                    .flex_col()
                    .child(div().flex_none().h(px(thumb_top)))
                    .child(
                        div()
                            .flex_none()
                            .w_full()
                            .h(px(thumb_height))
                            .rounded_full()
                            .bg(rgb(0x789094)),
                    )
                    .child(div().flex_1()),
            )
        })
}

fn settings_window_shell() -> gpui::Div {
    div()
        .size_full()
        .p_5()
        .flex()
        .flex_col()
        .bg(rgb(0xf4f6f4))
        .text_color(rgb(0x263238))
        .font_family(".SystemUIFont")
}

fn settings_window_header(title: &'static str, detail: &'static str) -> gpui::Div {
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
        .child(div().text_xs().text_color(rgb(0x6a7a81)).child(detail))
}

fn settings_window_header_with_action(
    header: impl IntoElement,
    action: impl IntoElement,
) -> gpui::Div {
    div()
        .flex()
        .items_start()
        .justify_between()
        .gap_3()
        .child(header)
        .child(action)
}

fn team_refresh_button(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .id("refresh-team-details")
        .flex_none()
        .h_8()
        .px_3()
        .rounded_md()
        .bg(rgb(0xebf1f0))
        .text_color(rgb(0x53666d))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xdfe8e6)))
        .on_click(cx.listener(SettingsWindow::refresh_team_details))
        .child(lucide_icon("icons/refresh-cw.svg"))
        .child("Refresh")
}

fn settings_label(label: &'static str) -> gpui::Div {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x66767c))
        .child(label)
}

fn developer_settings_section(
    title: &'static str,
    detail: &'static str,
    body: impl IntoElement,
) -> gpui::Div {
    div()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xd8e0df))
        .bg(rgb(0xffffff))
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x24333a))
                        .child(title),
                )
                .child(div().text_xs().text_color(rgb(0x6a7a81)).child(detail)),
        )
        .child(body)
}

fn settings_team_row(team: &TeamOption, selected: bool) -> gpui::Div {
    div()
        .h(px(COMBO_ITEM_HEIGHT))
        .px_3()
        .rounded_md()
        .border_1()
        .border_color(if selected {
            rgb(0x0f6f7a)
        } else {
            rgb(0xd8e0df)
        })
        .bg(if selected {
            rgb(0xf0fbfa)
        } else {
            rgb(0xffffff)
        })
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xf6f9f9)))
        .flex()
        .items_center()
        .child(combo_item_content(
            team.identifier.clone(),
            team.name.clone(),
            developer_account_title(&team.role),
        ))
}

fn app_id_section(
    team: &TeamOption,
    auto_app_id: bool,
    selected_app_id: usize,
    picker_open: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let manual_enabled = !auto_app_id;
    let selected_app_id = selected_app_id.min(team.app_ids.len().saturating_sub(1));
    let selected = team.app_ids.get(selected_app_id);

    developer_settings_section(
        "App ID",
        "Identifier used when creating or selecting the provisioning profile.",
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(auto_app_id_checkbox(auto_app_id, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(settings_label("Manual App ID"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when_some(selected, |this, selected| {
                                this.child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .when(!manual_enabled, |this| this.opacity(0.5))
                                        .child(combo_with_popover(
                                            "app-id-popover-scroll",
                                            combo_button(
                                                selected.identifier.clone(),
                                                selected.name.clone(),
                                                selected.kind.clone(),
                                                picker_open && manual_enabled,
                                            )
                                            .id("app-id-combobox")
                                            .when(
                                                manual_enabled,
                                                |this| {
                                                    this.on_click(cx.listener(
                                                        SettingsWindow::toggle_app_id_picker,
                                                    ))
                                                },
                                            ),
                                            picker_open && manual_enabled,
                                            dropdown_list((0..team.app_ids.len()).map(|index| {
                                                settings_app_id_row(
                                                    &team.app_ids[index],
                                                    index == selected_app_id,
                                                )
                                                .id(("app-id-row", index))
                                                .on_click(cx.listener(
                                                    move |this, event, window, cx| {
                                                        this.select_app_id(index, event, window, cx)
                                                    },
                                                ))
                                            }))
                                            .id("app-id-options"),
                                        )),
                                )
                            })
                            .child(app_id_actions(!manual_enabled, cx)),
                    ),
            ),
    )
}

fn auto_app_id_checkbox(checked: bool, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .id("auto-app-id")
        .p_2()
        .rounded_md()
        .bg(rgb(0xf6f9f9))
        .flex()
        .items_center()
        .gap_2()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xebf1f0)))
        .on_click(cx.listener(SettingsWindow::toggle_auto_app_id))
        .child(
            div()
                .flex_none()
                .w_4()
                .h_4()
                .rounded_sm()
                .border_1()
                .border_color(if checked {
                    rgb(0x173f45)
                } else {
                    rgb(0xaebcba)
                })
                .bg(if checked {
                    rgb(0x173f45)
                } else {
                    rgb(0xffffff)
                })
                .flex()
                .items_center()
                .justify_center()
                .when(checked, |this| {
                    this.child(div().w_2().h_2().rounded_sm().bg(rgb(0xffffff)))
                }),
        )
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_1()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x24333a))
                        .child("Automatically pick or create an App ID"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6a7a81))
                        .child("Manual App ID selection stays visible below."),
                ),
        )
}

fn app_id_actions(disabled: bool, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .child(app_id_icon_button(
            "add-app-id",
            "icons/plus.svg",
            disabled,
            cx.listener(SettingsWindow::add_app_id),
        ))
        .child(app_id_icon_button(
            "remove-app-id",
            "icons/minus.svg",
            disabled,
            cx.listener(SettingsWindow::remove_app_id),
        ))
        .child(app_id_icon_button(
            "edit-app-id",
            "icons/pencil.svg",
            disabled,
            cx.listener(SettingsWindow::edit_app_id),
        ))
}

fn app_id_icon_button(
    id: &'static str,
    icon: &'static str,
    disabled: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex_none()
        .w_6()
        .h_6()
        .rounded_md()
        .bg(rgb(0xebf1f0))
        .text_color(rgb(0x53666d))
        .flex()
        .items_center()
        .justify_center()
        .id(id)
        .when(disabled, |this| this.opacity(0.45))
        .when(!disabled, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(rgb(0xdfe8e6)))
                .on_click(listener)
        })
        .child(lucide_icon(icon))
}

fn settings_app_id_row(app_id: &AppIdOption, selected: bool) -> gpui::Div {
    div()
        .h(px(COMBO_ITEM_HEIGHT))
        .px_3()
        .rounded_md()
        .border_1()
        .border_color(if selected {
            rgb(0x0f6f7a)
        } else {
            rgb(0xd8e0df)
        })
        .bg(if selected {
            rgb(0xf0fbfa)
        } else {
            rgb(0xffffff)
        })
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xf6f9f9)))
        .flex()
        .items_center()
        .child(combo_item_content(
            app_id.identifier.clone(),
            app_id.name.clone(),
            app_id.kind.clone(),
        ))
}

fn settings_backend_combo_with_popover(
    trigger: impl IntoElement,
    open: bool,
    popover: impl IntoElement,
) -> gpui::Div {
    div().relative().child(trigger).when(open, |this| {
        this.child(
            div().absolute().top_0().left_0().w(px(0.)).h(px(0.)).child(
                deferred(
                    anchored()
                        .snap_to_window_with_margin(px(8.))
                        .position_mode(AnchoredPositionMode::Local)
                        .position(point(px(0.), px(ADI_BACKEND_COMBO_HEIGHT + 8.)))
                        .child(
                            div()
                                .w(px(ADI_BACKEND_POPOVER_WIDTH))
                                .max_h(px(ADI_BACKEND_POPOVER_MAX_HEIGHT))
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(0xd8e0df))
                                .bg(rgb(0xf8fbfa))
                                .id("adi-backend-popover-scroll")
                                .overflow_y_scroll()
                                .scrollbar_width(px(8.))
                                .occlude()
                                .child(popover),
                        ),
                )
                .with_priority(10),
            ),
        )
    })
}

fn settings_adi_backend_combo_button(backend: &AdiBackendOption, open: bool) -> gpui::Div {
    div()
        .h(px(ADI_BACKEND_COMBO_HEIGHT))
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(if open { rgb(0x0f6f7a) } else { rgb(0xcfd8d6) })
        .bg(rgb(0xffffff))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xf6f9f9)))
        .flex()
        .items_center()
        .gap_3()
        .child(status_dot(&backend.status))
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_1()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x24333a))
                        .text_ellipsis()
                        .child(backend.name.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6a7a81))
                        .text_ellipsis()
                        .child(backend.detail.clone()),
                ),
        )
        .child(status_badge(backend.status.clone(), false))
        .child(chevron(open))
}

fn settings_adi_backend_option(backend: &AdiBackendOption, selected: bool) -> gpui::Div {
    div()
        .min_h(px(56.))
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(if selected {
            rgb(0x0f6f7a)
        } else {
            rgb(0xd8e0df)
        })
        .bg(if selected {
            rgb(0xf0fbfa)
        } else {
            rgb(0xffffff)
        })
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xf6f9f9)))
        .flex()
        .items_center()
        .gap_2()
        .child(status_dot(&backend.status))
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
                        .text_ellipsis()
                        .child(backend.name.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6a7a81))
                        .text_ellipsis()
                        .child(backend.detail.clone()),
                ),
        )
        .child(status_badge(backend.status.clone(), selected))
}

fn adi_backend_status_section(
    backend: &AdiBackendOption,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let color = backend_status_color(&backend.status);

    div()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xd8e0df))
        .bg(rgb(0xffffff))
        .flex()
        .items_start()
        .gap_3()
        .child(
            div()
                .flex_none()
                .w(px(4.))
                .h_full()
                .min_h(px(76.))
                .rounded_full()
                .bg(rgb(color)),
        )
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_1()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(status_dot(&backend.status))
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0x24333a))
                                        .child("Status"),
                                ),
                        )
                        .child(status_badge(backend.status.clone(), false)),
                )
                .child(
                    div()
                        .min_w_0()
                        .w_full()
                        .text_xs()
                        .line_height(px(18.))
                        .text_color(rgb(0x6a7a81))
                        .whitespace_normal()
                        .child(backend.information.clone()),
                )
                .when(backend.repair_action.is_some(), |this| {
                    let label = backend
                        .repair_action
                        .clone()
                        .unwrap_or_else(|| "Fix Issues".into());
                    this.child(
                        div()
                            .id("repair-adi-backend")
                            .h_8()
                            .px_3()
                            .rounded_md()
                            .bg(rgb(color))
                            .text_color(rgb(0xffffff))
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|style| style.opacity(0.88))
                            .on_click(cx.listener(SettingsWindow::repair_adi_backend))
                            .child(label),
                    )
                }),
        )
}

fn machine_identity_section(
    identity: &MachineIdentity,
    editable: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    div()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xd8e0df))
        .bg(rgb(0xffffff))
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x24333a))
                        .child("Machine Identity"),
                )
                .when(!editable, |this| this.child(read_only_badge())),
        )
        .child(machine_identity_row(
            "Machine name",
            identity.machine_name.clone(),
            editable,
            cx,
        ))
        .child(machine_identity_row(
            "OS name",
            identity.os_name.clone(),
            editable,
            cx,
        ))
        .child(machine_identity_row(
            "OS version",
            identity.os_version.clone(),
            editable,
            cx,
        ))
        .child(machine_identity_row(
            "Machine ID",
            identity.machine_id.clone(),
            editable,
            cx,
        ))
}

fn machine_identity_row(
    label: &'static str,
    value: SharedString,
    editable: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(104.))
                .flex_none()
                .text_xs()
                .text_color(rgb(0x7b8a90))
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_sm()
                .text_color(rgb(0x405057))
                .text_ellipsis()
                .child(value),
        )
        .when(editable, |this| {
            this.child(
                div()
                    .id(("edit-machine-identity", machine_identity_id(label)))
                    .flex_none()
                    .w_6()
                    .h_6()
                    .rounded_md()
                    .bg(rgb(0xebf1f0))
                    .text_color(rgb(0x53666d))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0xdfe8e6)))
                    .on_click(cx.listener(SettingsWindow::edit_machine_identity))
                    .child(lucide_icon("icons/pencil.svg")),
            )
        })
}

fn machine_identity_id(label: &'static str) -> usize {
    match label {
        "Machine name" => 0,
        "OS name" => 1,
        "OS version" => 2,
        "Machine ID" => 3,
        _ => 99,
    }
}

fn status_badge(label: SharedString, selected: bool) -> gpui::Div {
    let color = backend_status_color(&label);

    div()
        .flex_none()
        .px_2()
        .h_6()
        .rounded_full()
        .bg(if selected {
            rgb(status_badge_selected_bg(&label))
        } else {
            rgb(status_badge_bg(&label))
        })
        .text_color(rgb(color))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .child(label)
}

fn status_dot(status: &SharedString) -> gpui::Div {
    div()
        .flex_none()
        .w_2()
        .h_2()
        .rounded_full()
        .bg(rgb(backend_status_color(status)))
}

fn backend_status_color(status: &SharedString) -> u32 {
    match status.to_string().as_str() {
        "Ready" => 0x1d6b45,
        "Needs setup" => 0x9a6a14,
        "Offline" => 0x9a302b,
        _ => 0x53666d,
    }
}

fn status_badge_bg(status: &SharedString) -> u32 {
    match status.to_string().as_str() {
        "Ready" => 0xe5f3ec,
        "Needs setup" => 0xf6edda,
        "Offline" => 0xf4e4e2,
        _ => 0xebf1f0,
    }
}

fn status_badge_selected_bg(status: &SharedString) -> u32 {
    match status.to_string().as_str() {
        "Ready" => 0xd3eadc,
        "Needs setup" => 0xeadbb8,
        "Offline" => 0xebcecb,
        _ => 0xd7efec,
    }
}

fn read_only_badge() -> gpui::Div {
    div()
        .px_2()
        .h_5()
        .rounded_full()
        .bg(rgb(0xf0f2f2))
        .text_color(rgb(0x718086))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .child("Read-only")
}

fn app_icon_editor(app: &AppOption, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    let initial = app
        .name
        .to_string()
        .chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "A".to_string());

    div()
        .flex_none()
        .w(px(84.))
        .h(px(84.))
        .rounded_md()
        .border_1()
        .border_color(rgb(0xcfd8d6))
        .bg(rgb(0x173f45))
        .text_color(rgb(0xffffff))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x20545c)))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .id("app-icon-editor")
        .on_click(cx.listener(SettingsWindow::edit_app_icon))
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child(initial),
        )
        .child(div().text_xs().child("Edit icon"))
}

fn app_detail_row(
    label: &'static str,
    value: String,
    editable: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(96.))
                .flex_none()
                .text_xs()
                .text_color(rgb(0x7b8a90))
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_sm()
                .text_color(rgb(0x405057))
                .text_ellipsis()
                .child(value),
        )
        .when(editable, |this| this.child(app_detail_controls(label, cx)))
}

fn app_detail_controls(label: &'static str, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .flex_none()
        .w(px(84.))
        .flex()
        .items_center()
        .justify_end()
        .gap_1()
        .child(app_detail_revert_slot(label, false, cx))
        .child(edit_button(label, cx))
}

fn edit_button(label: &'static str, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .flex_none()
        .w_6()
        .h_6()
        .rounded_md()
        .bg(rgb(0xebf1f0))
        .text_color(rgb(0x53666d))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xdfe8e6)))
        .id(("edit-app-detail", app_detail_id(label)))
        .on_click(cx.listener(SettingsWindow::edit_app_detail))
        .child(lucide_icon("icons/pencil.svg"))
}

fn app_detail_revert_slot(
    label: &'static str,
    changed: bool,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    div()
        .flex_none()
        .w(px(52.))
        .h_6()
        .rounded_md()
        .bg(rgb(0xebf1f0))
        .text_color(rgb(0x53666d))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .id(("revert-app-detail", app_detail_id(label)))
        .when(!changed, |this| this.opacity(0.))
        .when(changed, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(rgb(0xdfe8e6)))
                .on_click(cx.listener(SettingsWindow::revert_app_detail))
        })
        .child("Revert")
}

fn app_detail_id(label: &'static str) -> usize {
    match label {
        "Name" => 0,
        "Bundle ID" => 1,
        "Version" => 2,
        "Build" => 3,
        "Executable" => 4,
        "Minimum OS" => 5,
        "Supported devices" => 6,
        _ => 99,
    }
}

fn enabled_patch_summary(app: &AppOption, enabled_patches: &[bool]) -> String {
    let enabled = app
        .patches
        .iter()
        .enumerate()
        .filter_map(|(index, patch)| {
            enabled_patches
                .get(index)
                .copied()
                .filter(|enabled| *enabled)
                .map(|_| format!("{} ({})", patch.name, patch.detail))
        })
        .collect::<Vec<_>>();

    if enabled.is_empty() {
        "No patch changes selected".to_string()
    } else {
        enabled.join(", ")
    }
}

fn entitlements_table(app: &AppOption, cx: &mut Context<SettingsWindow>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xd8e0df))
        .bg(rgb(0xffffff))
        .overflow_hidden()
        .child(
            div()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(rgb(0xd8e0df))
                .flex()
                .items_center()
                .justify_start()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x24333a))
                        .child("Entitlements"),
                )
                .child(entitlements_actions(false, cx)),
        )
        .child(entitlement_header_row())
        .child(entitlement_row(
            "application-identifier",
            "String",
            format!("TEAMID1234.{}", app.bundle_id),
        ))
        .child(entitlement_row(
            "com.apple.developer.team-identifier",
            "String",
            "TEAMID1234".to_string(),
        ))
        .child(entitlement_row(
            "get-task-allow",
            "Boolean",
            "true".to_string(),
        ))
        .child(entitlement_row(
            "keychain-access-groups",
            "Array",
            format!("TEAMID1234.{}", app.bundle_id),
        ))
        .child(entitlement_row(
            "com.apple.security.application-groups",
            "Array",
            "None".to_string(),
        ))
}

fn entitlements_actions(changed: bool, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .child(entitlement_icon_button(
            "add-entitlement",
            "icons/plus.svg",
            cx.listener(SettingsWindow::add_entitlement),
        ))
        .child(entitlement_icon_button(
            "remove-entitlement",
            "icons/minus.svg",
            cx.listener(SettingsWindow::remove_entitlement),
        ))
        .child(entitlements_revert_slot(changed, cx))
}

fn entitlement_icon_button(
    id: &'static str,
    icon: &'static str,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex_none()
        .w_6()
        .h_6()
        .rounded_md()
        .bg(rgb(0xebf1f0))
        .text_color(rgb(0x53666d))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xdfe8e6)))
        .id(id)
        .on_click(listener)
        .child(lucide_icon(icon))
}

fn entitlements_revert_slot(changed: bool, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .flex_none()
        .w(px(52.))
        .h_6()
        .rounded_md()
        .bg(rgb(0xebf1f0))
        .text_color(rgb(0x53666d))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .id("revert-entitlements")
        .when(!changed, |this| this.opacity(0.))
        .when(changed, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(rgb(0xdfe8e6)))
                .on_click(cx.listener(SettingsWindow::revert_entitlements))
        })
        .child("Revert")
}

fn entitlement_header_row() -> gpui::Div {
    div()
        .grid()
        .grid_cols(3)
        .px_3()
        .py_2()
        .bg(rgb(0xf6f9f9))
        .border_b_1()
        .border_color(rgb(0xd8e0df))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x66767c))
        .child("Key")
        .child("Type")
        .child("Value")
}

fn entitlement_row(key: &'static str, kind: &'static str, value: String) -> gpui::Div {
    div()
        .grid()
        .grid_cols(3)
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(rgb(0xe8eeee))
        .text_xs()
        .text_color(rgb(0x405057))
        .child(div().min_w_0().text_ellipsis().child(key))
        .child(div().min_w_0().text_ellipsis().child(kind))
        .child(div().min_w_0().text_ellipsis().child(value))
}
