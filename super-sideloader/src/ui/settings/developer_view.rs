use super::*;
use gpui_component::{checkbox::Checkbox, switch::Switch, Sizable as _};

pub(super) struct DeveloperViewProps<'a> {
    pub(super) focus_handle: &'a FocusHandle,
    pub(super) scroll_handle: &'a ScrollHandle,
    pub(super) teams: &'a [TeamOption],
    pub(super) selected_team: usize,
    pub(super) selected_certificate: usize,
    pub(super) auto_app_id: bool,
    pub(super) selected_app_id: usize,
    pub(super) team_picker_open: bool,
    pub(super) certificate_picker_open: bool,
    pub(super) app_id_picker_open: bool,
    pub(super) app_id_add_form: Option<&'a AppIdAddForm>,
    pub(super) app_id_edit_form: Option<&'a AppIdEditForm>,
    pub(super) team_refreshing: bool,
    pub(super) team_refresh_error: Option<SharedString>,
    pub(super) certificate_error: Option<SharedString>,
    pub(super) spinner_turns: f32,
}

pub(super) fn render(props: DeveloperViewProps<'_>, cx: &mut Context<SettingsWindow>) -> gpui::Div {
    let DeveloperViewProps {
        focus_handle,
        scroll_handle,
        teams,
        selected_team,
        selected_certificate,
        auto_app_id,
        selected_app_id,
        team_picker_open,
        certificate_picker_open,
        app_id_picker_open,
        app_id_add_form,
        app_id_edit_form,
        team_refreshing,
        team_refresh_error,
        certificate_error,
        spinner_turns,
    } = props;
    let selected = selected_team;
    let team = teams.get(selected);
    let mut content =
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(team_section(teams, selected, team_picker_open, cx));

    content = content.child(certificate_section(
        team,
        selected_certificate,
        certificate_picker_open,
        team_refreshing,
        certificate_error,
        cx,
    ));
    content = content.child(app_id_section(
        AppIdSectionProps {
            team,
            auto_app_id,
            selected_app_id,
            picker_open: app_id_picker_open,
            add_form: app_id_add_form,
            edit_form: app_id_edit_form,
            refreshing: team_refreshing,
            operation_error: team_refresh_error,
        },
        cx,
    ));
    content = content.child(developer_logout_button(cx));

    settings_window_shell()
        .track_focus(focus_handle)
        .capture_key_down(cx.listener(SettingsWindow::handle_machine_identity_editor_key))
        .gap_4()
        .child(settings_window_header_with_action(
            settings_window_header(
                "Developer Settings",
                "Configure developer-team resources used during signing.",
            ),
            team_refresh_button(team_refreshing, spinner_turns, cx),
        ))
        .child(scroll_panel("team-settings-scroll", scroll_handle, content))
}

fn refresh_error(error: SharedString) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xe7c8c5))
        .bg(rgb(0xfff7f6))
        .p_3()
        .text_xs()
        .text_color(rgb(0x8f3b35))
        .child(error)
}

fn team_refresh_button(
    refreshing: bool,
    spinner_turns: f32,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    surface_button("refresh-team-details")
        .flex_none()
        .h_8()
        .when(!refreshing, |this| {
            this.cursor_pointer()
                .on_click(cx.listener(SettingsWindow::refresh_team_details))
        })
        .when(refreshing, |this| this.opacity(0.55).tab_stop(false))
        .child(action_button_surface(
            "Refresh",
            0xebf1f0,
            0xdfe8e6,
            0x53666d,
            Some(if refreshing {
                div()
                    .w_4()
                    .h_4()
                    .child(progress_circle(0.34, spinner_turns))
                    .into_any_element()
            } else {
                lucide_icon_tinted("icons/refresh-cw.svg", 0x53666d).into_any_element()
            }),
        ))
}

fn action_card_surface(
    title: &'static str,
    detail: &'static str,
    icon: &'static str,
    text_color: u32,
    icon_bg: u32,
    hover_bg: u32,
) -> gpui::Div {
    div()
        .min_w_0()
        .size_full()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xe0d3d1))
        .bg(rgb(0xffffff))
        .hover(|style| style.bg(rgb(hover_bg)))
        .text_color(rgb(text_color))
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .min_w_0()
                .w_full()
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
                                .child(title),
                        )
                        .child(div().text_xs().text_color(rgb(0x8b6b67)).child(detail)),
                )
                .child(
                    div()
                        .flex_none()
                        .w_7()
                        .h_7()
                        .rounded_md()
                        .bg(rgb(icon_bg))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(lucide_icon(icon)),
                ),
        )
}

fn developer_logout_button(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    surface_button("developer-log-out")
        .w_full()
        .cursor_pointer()
        .on_click(cx.listener(SettingsWindow::log_out_developer_account))
        .child(action_card_surface(
            "Log Out",
            "Remove this Apple Account session from Super Sideloader.",
            "icons/log-out.svg",
            0x7d3430,
            0xf4e9e7,
            0xfff7f6,
        ))
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

fn disabled_select_placeholder(
    eyebrow: &'static str,
    title: &'static str,
    detail: &'static str,
) -> impl IntoElement {
    div()
        .min_h(px(72.))
        .rounded_md()
        .border_1()
        .border_color(rgb(0xd8e0df))
        .bg(rgb(0xf6f9f9))
        .px_3()
        .flex()
        .items_center()
        .child(select_item_content(eyebrow, title, detail))
}

fn empty_state(message: &'static str) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xd8e0df))
        .bg(rgb(0xf6f9f9))
        .p_3()
        .text_xs()
        .text_color(rgb(0x6a7a81))
        .child(message)
}

fn team_section(
    teams: &[TeamOption],
    selected: usize,
    picker_open: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let team = teams.get(selected);
    developer_settings_section(
        "Developer Team",
        "Team used for certificates, profiles, and signing identities.",
        div()
            .flex()
            .flex_col()
            .gap_3()
            .when_some(team, |this, team| {
                this.child(select_with_popover(
                    "team-popover-scroll",
                    select_button(
                        "settings-team-select",
                        team.identifier.clone(),
                        team.name.clone(),
                        developer_account_title(&team.role),
                        picker_open,
                    ),
                    picker_open,
                    cx.listener(|this, open: &bool, _, cx| {
                        if matches!(this.mode, SettingsMode::Team) {
                            this.team_picker_open = *open;
                            if *open {
                                this.certificate_picker_open = false;
                                this.app_id_picker_open = false;
                            }
                            cx.notify();
                        }
                    }),
                    dropdown_list((0..teams.len()).map(|index| {
                        settings_team_row(&teams[index], index, index == selected).on_click(
                            cx.listener(move |this, event, window, cx| {
                                this.select_team(index, event, window, cx)
                            }),
                        )
                    }))
                    .id("settings-team-options"),
                ))
                .child(properties_list(vec![
                    ("Name", team.name.to_string()),
                    ("Team ID", team.identifier.to_string()),
                    ("Type", team.role.to_string()),
                ]))
            })
            .when(team.is_none(), |this| {
                this.child(disabled_select_placeholder(
                    "No developer team",
                    "Refresh required",
                    "No team data has been loaded.",
                ))
                .child(empty_state("Refresh developer resources to load teams."))
            }),
    )
}

fn settings_team_row(team: &TeamOption, index: usize, selected: bool) -> Button {
    select_option_button(
        ("settings-team-row", index),
        selected,
        div()
            .min_w_0()
            .w_full()
            .flex()
            .items_center()
            .child(select_item_content(
                team.identifier.clone(),
                team.name.clone(),
                developer_account_title(&team.role),
            )),
    )
}

fn certificate_section(
    team: Option<&TeamOption>,
    selected_certificate: usize,
    picker_open: bool,
    refreshing: bool,
    operation_error: Option<SharedString>,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let certificates: &[DevelopmentCertificateOption] =
        team.map(|team| team.certificates.as_slice()).unwrap_or(&[]);
    let selected_certificate = team
        .map(|team| selected_certificate.min(team.certificates.len().saturating_sub(1)))
        .unwrap_or(0);
    let selected = certificates.get(selected_certificate);
    let unavailable = team.is_none() || selected.is_none();

    developer_settings_section(
        "Certificates",
        "Development certificates available for signing with this team.",
        div()
            .flex()
            .flex_col()
            .gap_3()
            .when_some(selected, |this, certificate| {
                let trigger = select_button(
                    "certificate-select",
                    certificate.serial_number.clone(),
                    certificate.name.clone(),
                    certificate.detail(),
                    picker_open && !refreshing,
                )
                .when(refreshing, |this| this.tab_stop(false));

                this.child(
                    div()
                        .min_w_0()
                        .w_full()
                        .when(refreshing, |this| this.opacity(0.55))
                        .child(if refreshing {
                            trigger.into_any_element()
                        } else {
                            select_with_popover(
                                "certificate-popover-scroll",
                                trigger,
                                picker_open,
                                cx.listener(|this, open: &bool, _, cx| {
                                    if matches!(this.mode, SettingsMode::Team) {
                                        this.certificate_picker_open = *open;
                                        if *open {
                                            this.team_picker_open = false;
                                            this.app_id_picker_open = false;
                                        }
                                        cx.notify();
                                    }
                                }),
                                dropdown_list((0..certificates.len()).map(|index| {
                                    settings_certificate_row(
                                        &certificates[index],
                                        index,
                                        index == selected_certificate,
                                    )
                                    .on_click(cx.listener(
                                        move |this, event, window, cx| {
                                            this.select_certificate(index, event, window, cx)
                                        },
                                    ))
                                }))
                                .id("certificate-options"),
                            )
                            .into_any_element()
                        }),
                )
                .child(certificate_details(certificate))
            })
            .when(selected.is_none(), |this| {
                this.child(disabled_select_placeholder(
                    "No certificate",
                    if team.is_some() {
                        "No development certificate is available."
                    } else {
                        "Load a developer team first."
                    },
                    "Signing certificate unavailable",
                ))
                .child(create_certificate_button(team.is_none() || refreshing, cx))
            })
            .when_some(
                selected.filter(|certificate| !certificate.private_key_available),
                |this, _| this.child(missing_private_key_warning(refreshing, cx)),
            )
            .when(!unavailable, |this| {
                this.child(certificate_actions(refreshing, cx))
            })
            .when_some(operation_error, |this, error| {
                this.child(refresh_error(error))
            }),
    )
}

fn certificate_details(certificate: &DevelopmentCertificateOption) -> impl IntoElement {
    properties_list(vec![
        ("Private key", certificate.private_key_status().to_string()),
        ("Serial", certificate.serial_number.to_string()),
        ("Certificate ID", certificate.id.to_string()),
        (
            "Machine",
            if certificate.machine_name.trim().is_empty() {
                "Unknown".to_string()
            } else {
                certificate.machine_name.to_string()
            },
        ),
    ])
}

fn create_certificate_button(disabled: bool, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    surface_button("create-certificate")
        .h_8()
        .when(disabled, |this| this.opacity(0.45).tab_stop(false))
        .when(!disabled, |this| {
            this.cursor_pointer()
                .on_click(cx.listener(SettingsWindow::create_certificate))
        })
        .child(primary_action_button_surface(
            "Create Certificate",
            Some(lucide_icon_tinted("icons/plus.svg", 0xffffff)),
        ))
}

fn missing_private_key_warning(
    refreshing: bool,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xe6d7bc))
        .bg(rgb(0xfffbf3))
        .p_3()
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
                        .text_color(rgb(0x7a5613))
                        .child("Private key missing"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x7a6a50))
                        .child("Import the matching private key to use this certificate."),
                ),
        )
        .child(
            surface_button("import-certificate-key")
                .flex_none()
                .h_7()
                .when(refreshing, |this| this.opacity(0.45).tab_stop(false))
                .when(!refreshing, |this| {
                    this.cursor_pointer()
                        .on_click(cx.listener(SettingsWindow::import_certificate_private_key))
                })
                .child(action_button_surface(
                    "Import Key",
                    0xebf1f0,
                    0xdfe8e6,
                    0x53666d,
                    Some(lucide_icon_tinted("icons/key-round.svg", 0x53666d)),
                )),
        )
}

fn certificate_actions(refreshing: bool, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div().flex().items_center().justify_end().child(
        surface_button("revoke-certificate")
            .h_7()
            .when(refreshing, |this| this.opacity(0.45).tab_stop(false))
            .when(!refreshing, |this| {
                this.cursor_pointer()
                    .on_click(cx.listener(SettingsWindow::revoke_certificate))
            })
            .child(action_button_surface(
                "Revoke",
                0xf4e9e7,
                0xfff7f6,
                0x7d3430,
                Some(lucide_icon_tinted("icons/trash-2.svg", 0x7d3430)),
            )),
    )
}

fn settings_certificate_row(
    certificate: &DevelopmentCertificateOption,
    index: usize,
    selected: bool,
) -> Button {
    select_option_button(
        ("certificate-row", index),
        selected,
        div()
            .min_w_0()
            .w_full()
            .flex()
            .items_center()
            .child(select_item_content(
                certificate.serial_number.clone(),
                certificate.name.clone(),
                certificate.detail(),
            )),
    )
}

struct AppIdSectionProps<'a> {
    team: Option<&'a TeamOption>,
    auto_app_id: bool,
    selected_app_id: usize,
    picker_open: bool,
    add_form: Option<&'a AppIdAddForm>,
    edit_form: Option<&'a AppIdEditForm>,
    refreshing: bool,
    operation_error: Option<SharedString>,
}

fn app_id_section(props: AppIdSectionProps<'_>, cx: &mut Context<SettingsWindow>) -> gpui::Div {
    let AppIdSectionProps {
        team,
        auto_app_id,
        selected_app_id,
        picker_open,
        add_form,
        edit_form,
        refreshing,
        operation_error,
    } = props;
    let manual_enabled = !auto_app_id && team.is_some();
    let app_ids: &[AppIdOption] = team.map(|team| team.app_ids.as_slice()).unwrap_or(&[]);
    let selected_app_id = team
        .map(|team| selected_app_id.min(team.app_ids.len().saturating_sub(1)))
        .unwrap_or(0);
    let selected = app_ids.get(selected_app_id);
    let add_open = add_form.is_some();
    let edit_open = edit_form.is_some();
    let add_disabled = !manual_enabled || refreshing;
    let selected_app_id_available = selected.is_some();
    let remove_disabled = !manual_enabled || refreshing || !selected_app_id_available;
    let edit_disabled = !manual_enabled || refreshing || !selected_app_id_available;
    let save_disabled = !manual_enabled || refreshing || !selected_app_id_available;
    let quota = team.and_then(app_id_quota_text);

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
                    .child(app_id_manual_header(quota.as_deref()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when_some(selected, |this, selected| {
                                let trigger = select_button(
                                    "app-id-select",
                                    selected.identifier.clone(),
                                    selected.name.clone(),
                                    selected.kind.clone(),
                                    picker_open && manual_enabled,
                                )
                                .when(!manual_enabled, |this| this.tab_stop(false));

                                this.child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .when(!manual_enabled, |this| this.opacity(0.5))
                                        .child(if manual_enabled {
                                            select_with_popover(
                                                "app-id-popover-scroll",
                                                trigger,
                                                picker_open,
                                                cx.listener(move |this, open: &bool, _, cx| {
                                                    if manual_enabled
                                                        && matches!(this.mode, SettingsMode::Team)
                                                    {
                                                        this.app_id_picker_open = *open;
                                                        if *open {
                                                            this.team_picker_open = false;
                                                            this.certificate_picker_open = false;
                                                        }
                                                        cx.notify();
                                                    }
                                                }),
                                                dropdown_list((0..app_ids.len()).map(|index| {
                                                    settings_app_id_row(
                                                        &app_ids[index],
                                                        index,
                                                        index == selected_app_id,
                                                    )
                                                    .on_click(cx.listener(
                                                        move |this, event, window, cx| {
                                                            this.select_app_id(
                                                                index, event, window, cx,
                                                            )
                                                        },
                                                    ))
                                                }))
                                                .id("app-id-options"),
                                            )
                                            .into_any_element()
                                        } else {
                                            trigger.into_any_element()
                                        }),
                                )
                            })
                            .when(selected.is_none(), |this| {
                                this.child(div().min_w_0().flex_1().opacity(0.5).child(
                                    disabled_select_placeholder(
                                        "No App ID",
                                        if team.is_some() {
                                            "No manual App ID is available."
                                        } else {
                                            "Load a developer team first."
                                        },
                                        "Manual selection unavailable",
                                    ),
                                ))
                            })
                            .child(app_id_actions(
                                AppIdActionsProps {
                                    add_disabled,
                                    add_open,
                                    add_form,
                                    remove_disabled,
                                    edit_open,
                                    edit_form,
                                    edit_disabled,
                                    save_disabled,
                                },
                                cx,
                            )),
                    ),
            )
            .when_some(quota, |this, _| this.child(app_id_quota_warning()))
            .when_some(operation_error, |this, error| {
                this.child(refresh_error(error))
            }),
    )
}

fn app_id_quota_text(team: &TeamOption) -> Option<String> {
    let available = team.app_id_available_quantity?;
    Some(match team.app_id_max_quantity {
        Some(max) => format!("{available} of {max}"),
        None => available.to_string(),
    })
}

fn app_id_manual_header(quota: Option<&str>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(settings_label("Manual App ID"))
        .when_some(quota, |this, quota| this.child(app_id_quota_badge(quota)))
}

fn app_id_quota_badge(quota: &str) -> impl IntoElement {
    div()
        .flex_none()
        .px_2()
        .py_0p5()
        .rounded_full()
        .bg(rgb(0xebf1f0))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x53666d))
        .child(format!("{quota} remaining"))
}

fn app_id_quota_warning() -> impl IntoElement {
    div()
        .px_1()
        .text_xs()
        .text_color(rgb(0x6a7a81))
        .line_height(px(18.))
        .child("Deleting App IDs does not increase the remaining App ID quota.")
}

fn auto_app_id_checkbox(checked: bool, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    Checkbox::new("auto-app-id-checkbox")
        .p_2()
        .rounded_md()
        .bg(rgb(0xf6f9f9))
        .hover(|style| style.bg(rgb(0xebf1f0)))
        .items_center()
        .small()
        .checked(checked)
        .on_click(cx.listener(SettingsWindow::set_auto_app_id))
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

struct AppIdActionsProps<'a> {
    add_disabled: bool,
    add_open: bool,
    add_form: Option<&'a AppIdAddForm>,
    remove_disabled: bool,
    edit_open: bool,
    edit_form: Option<&'a AppIdEditForm>,
    edit_disabled: bool,
    save_disabled: bool,
}

fn app_id_actions(
    props: AppIdActionsProps<'_>,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    let AppIdActionsProps {
        add_disabled,
        add_open,
        add_form,
        remove_disabled,
        edit_open,
        edit_form,
        edit_disabled,
        save_disabled,
    } = props;
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .child(app_id_add_button(add_disabled, add_open, add_form, cx))
        .child(app_id_icon_button(
            "remove-app-id",
            "icons/minus.svg",
            remove_disabled,
            cx.listener(SettingsWindow::remove_app_id),
        ))
        .child(app_id_edit_button(edit_disabled, edit_open, edit_form, cx))
        .child(app_id_icon_button(
            "save-mobileprovision",
            "icons/download.svg",
            save_disabled,
            cx.listener(SettingsWindow::save_mobileprovision),
        ))
}

fn app_id_add_button(
    disabled: bool,
    open: bool,
    form: Option<&AppIdAddForm>,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let trigger = surface_button("add-app-id")
        .flex_none()
        .w_6()
        .h_6()
        .child(icon_button_surface(
            "icons/plus.svg",
            0xebf1f0,
            0xdfe8e6,
            0x53666d,
        ));

    if disabled {
        return trigger.opacity(0.45).tab_stop(false).into_any_element();
    }

    floating_select_popover(
        "app-id-add-popover-scroll",
        trigger.cursor_pointer(),
        open,
        cx.listener(|this, open: &bool, window, cx| {
            this.set_app_id_add_popover(*open, window, cx);
        }),
        FloatingPopoverLayout {
            width: 420.,
            max_height: 260.,
            offset_y: 34.,
        },
        app_id_add_popover(form, cx),
    )
    .into_any_element()
}

fn app_id_add_popover(
    form: Option<&AppIdAddForm>,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    let Some(form) = form else {
        return div()
            .p_3()
            .text_xs()
            .text_color(rgb(0x6a7a81))
            .child("Preparing App ID...");
    };

    div()
        .p_3()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(settings_label("Identifier"))
                .child(Input::new(&form.identifier).small()),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(settings_label("Name"))
                .child(Input::new(&form.name).small()),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .child(
                    surface_button("cancel-app-id-add")
                        .h_7()
                        .cursor_pointer()
                        .on_click(cx.listener(SettingsWindow::cancel_app_id_add))
                        .child(action_button_surface(
                            "Cancel",
                            0xebf1f0,
                            0xdfe8e6,
                            0x53666d,
                            None::<gpui::Div>,
                        )),
                )
                .child(
                    surface_button("confirm-app-id-add")
                        .h_7()
                        .cursor_pointer()
                        .on_click(cx.listener(SettingsWindow::add_app_id))
                        .child(primary_action_button_surface(
                            "Add",
                            Some(lucide_icon_tinted("icons/plus.svg", 0xffffff)),
                        )),
                ),
        )
}

fn app_id_edit_button(
    disabled: bool,
    open: bool,
    form: Option<&AppIdEditForm>,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let trigger = surface_button("edit-app-id")
        .flex_none()
        .w_6()
        .h_6()
        .child(icon_button_surface(
            "icons/pencil.svg",
            0xebf1f0,
            0xdfe8e6,
            0x53666d,
        ));

    if disabled {
        return trigger.opacity(0.45).tab_stop(false).into_any_element();
    }

    floating_select_popover(
        "app-id-edit-popover-scroll",
        trigger.cursor_pointer(),
        open,
        cx.listener(|this, open: &bool, window, cx| {
            this.set_app_id_edit_popover(*open, window, cx);
        }),
        FloatingPopoverLayout {
            width: 520.,
            max_height: 520.,
            offset_y: 34.,
        },
        app_id_edit_popover(form, cx),
    )
    .into_any_element()
}

fn app_id_edit_popover(
    form: Option<&AppIdEditForm>,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    let Some(form) = form else {
        return div()
            .p_3()
            .text_xs()
            .text_color(rgb(0x6a7a81))
            .child("Preparing App ID...");
    };

    div()
        .p_3()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(settings_label("Name"))
                .child(Input::new(&form.name).small()),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_label("Capabilities"))
                .children(
                    form.capabilities
                        .iter()
                        .enumerate()
                        .map(|(index, capability)| app_id_capability_row(index, capability, cx)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .child(
                    surface_button("cancel-app-id-edit")
                        .h_7()
                        .cursor_pointer()
                        .on_click(cx.listener(SettingsWindow::cancel_app_id_edit))
                        .child(action_button_surface(
                            "Cancel",
                            0xebf1f0,
                            0xdfe8e6,
                            0x53666d,
                            None::<gpui::Div>,
                        )),
                )
                .child(
                    surface_button("confirm-app-id-edit")
                        .h_7()
                        .cursor_pointer()
                        .on_click(cx.listener(SettingsWindow::submit_app_id_update))
                        .child(primary_action_button_surface(
                            "Apply",
                            Some(lucide_icon_tinted("icons/check.svg", 0xffffff)),
                        )),
                ),
        )
}

fn app_id_capability_row(
    index: usize,
    capability: &AppIdCapabilityEdit,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    let enabled = capability.enabled;
    surface_button(("app-id-capability-row", index))
        .h_12()
        .w_full()
        .cursor_pointer()
        .on_click(cx.listener(move |settings, _, window, cx| {
            settings.toggle_app_id_capability(index, !enabled, window, cx)
        }))
        .child(
            div()
                .size_full()
                .px_3()
                .rounded_md()
                .border_1()
                .border_color(if enabled {
                    rgb(0x168291)
                } else {
                    rgb(0xd8e0df)
                })
                .bg(if enabled {
                    rgb(0xf0fbfa)
                } else {
                    rgb(0xffffff)
                })
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
                                .text_color(rgb(0x24333a))
                                .text_ellipsis()
                                .child(capability.label.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x6a7a81))
                                .text_ellipsis()
                                .child(capability.detail.clone()),
                        ),
                )
                .child(
                    div().flex_none().child(
                        Switch::new(("app-id-capability-switch", index))
                            .checked(enabled)
                            .small(),
                    ),
                ),
        )
}

fn app_id_icon_button(
    id: &'static str,
    icon: &'static str,
    disabled: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    surface_button(id)
        .flex_none()
        .w_6()
        .h_6()
        .when(disabled, |this| this.opacity(0.45).tab_stop(false))
        .when(!disabled, |this| this.cursor_pointer().on_click(listener))
        .child(icon_button_surface(icon, 0xebf1f0, 0xdfe8e6, 0x53666d))
}

fn settings_app_id_row(app_id: &AppIdOption, index: usize, selected: bool) -> Button {
    select_option_button(
        ("app-id-row", index),
        selected,
        div()
            .min_w_0()
            .w_full()
            .flex()
            .items_center()
            .child(select_item_content(
                app_id.identifier.clone(),
                app_id.name.clone(),
                app_id.kind.clone(),
            )),
    )
}
