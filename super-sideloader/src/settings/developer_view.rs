use super::*;

pub(super) fn render(
    focus_handle: &FocusHandle,
    scroll_handle: &ScrollHandle,
    teams: &[TeamOption],
    selected_team: usize,
    auto_app_id: bool,
    selected_app_id: usize,
    team_picker_open: bool,
    app_id_picker_open: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let selected = selected_team;
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
                        .on_click(cx.listener(SettingsWindow::toggle_team_picker)),
                        team_picker_open,
                        dropdown_list((0..teams.len()).map(|index| {
                            settings_team_row(&teams[index], index == selected)
                                .id(("settings-team-row", index))
                                .on_click(cx.listener(move |this, event, window, cx| {
                                    this.select_team(index, event, window, cx)
                                }))
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
            team_refresh_button(cx),
        ))
        .child(scroll_panel("team-settings-scroll", scroll_handle, content))
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

fn developer_logout_button(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .id("developer-log-out")
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xe0d3d1))
        .bg(rgb(0xffffff))
        .text_color(rgb(0x7d3430))
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xfff7f6)))
        .on_click(cx.listener(SettingsWindow::log_out_developer_account))
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
                        .child("Log Out"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x8b6b67))
                        .child("Remove this Apple Account session from Super Sideloader."),
                ),
        )
        .child(
            div()
                .flex_none()
                .w_7()
                .h_7()
                .rounded_md()
                .bg(rgb(0xf4e9e7))
                .flex()
                .items_center()
                .justify_center()
                .child(lucide_icon("icons/log-out.svg")),
        )
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
