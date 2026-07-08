use super::*;

pub(super) fn render(
    focus_handle: &FocusHandle,
    scroll_handle: &ScrollHandle,
    backends: &[AdiBackendOption],
    selected_backend: usize,
    machine_identity: &MachineIdentity,
    android_device_identity: &MachineIdentity,
    adi_backend_picker_open: bool,
    adi_operation: Option<&AdiOperationState>,
    spinner_turns: f32,
    machine_identity_edit: Option<&MachineIdentityEdit>,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let selected = selected_backend.min(backends.len().saturating_sub(1));
    let backend = backends.get(selected);
    let displayed_machine_identity: &MachineIdentity = match backend.map(|backend| backend.kind) {
        Some(AdiBackendKind::AndroidCoreAdi) => android_device_identity,
        Some(AdiBackendKind::SystemAdid) | Some(AdiBackendKind::WindowsCoreAdi) | None => {
            machine_identity
        }
    };
    let editable_identity = backend
        .map(|backend| backend.editable_identity)
        .unwrap_or(false);
    let backend_controls_disabled = adi_operation.is_some();

    let mut content = div().flex().flex_col().gap_4().child(
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
                    settings_adi_backend_combo_button(backend, adi_backend_picker_open)
                        .id("adi-backend-combobox")
                        .when(backend_controls_disabled, |this| this.opacity(0.55))
                        .when(!backend_controls_disabled, |this| {
                            this.on_click(cx.listener(SettingsWindow::toggle_adi_backend_picker))
                        }),
                    adi_backend_picker_open,
                    dropdown_list((0..backends.len()).map(|index| {
                        settings_adi_backend_option(&backends[index], index == selected)
                            .id(("adi-backend-option", index))
                            .on_click(cx.listener(move |this, event, window, cx| {
                                this.select_adi_backend(index, event, window, cx)
                            }))
                    }))
                    .id("adi-backend-options"),
                ))
            }),
    );

    if let Some(backend) = backend {
        content = content.child(adi_backend_status_section(
            backend,
            adi_operation,
            spinner_turns,
            cx,
        ));
    }

    content = content.child(
        machine_identity_section(
            displayed_machine_identity,
            editable_identity && !backend_controls_disabled,
            machine_identity_edit,
            cx,
        )
        .when(backend_controls_disabled, |this| this.opacity(0.55)),
    );

    settings_window_shell()
        .track_focus(focus_handle)
        .gap_4()
        .child(settings_window_header_with_action(
            settings_window_header(
                "Settings",
                "Choose the ADI backend and review the machine identity it uses.",
            ),
            open_data_folder_button(cx),
        ))
        .child(scroll_panel("adi-settings-scroll", scroll_handle, content))
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
    let status = backend.availability.label();

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
        .child(status_dot(&status))
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
        .child(status_badge(status, false))
        .child(chevron(open))
}

fn settings_adi_backend_option(backend: &AdiBackendOption, selected: bool) -> gpui::Div {
    let status = backend.availability.label();

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
        .child(status_dot(&status))
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
        .child(status_badge(status, selected))
}

fn adi_backend_status_section(
    backend: &AdiBackendOption,
    adi_operation: Option<&AdiOperationState>,
    spinner_turns: f32,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let status = backend.availability.label();
    let color = backend_status_color(&status);
    let coreadi_operation = adi_operation.filter(|operation| operation.is_coreadi_install());
    let provisioning_operation = adi_operation.filter(|operation| operation.is_provisioning());
    let provisioning_label = if provisioning_operation.is_some() {
        "Provisioning".into()
    } else {
        backend.provisioning_state.label()
    };

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
                                .child(status_dot(&status))
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0x24333a))
                                        .child("Status"),
                                ),
                        )
                        .child(status_badge(status, false)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(settings_label("Provisioning"))
                        .child(status_badge(provisioning_label, false)),
                )
                .when_some(backend.provisioning_state.detail(), |this, detail| {
                    this.child(
                        div()
                            .min_w_0()
                            .text_xs()
                            .line_height(px(17.))
                            .text_color(rgb(0x8a4a46))
                            .whitespace_normal()
                            .child(detail),
                    )
                })
                .children(backend.details.iter().map(|detail| {
                    adi_backend_detail_row(detail.label.clone(), detail.value.clone())
                }))
                .when(
                    matches!(
                        &backend.provisioning_state,
                        AdiProvisioningState::Provisioned
                    ),
                    |this| {
                        this.child(adi_status_button(
                            "erase-adi-provisioning",
                            "Erase Provisioning",
                            false,
                            0xf4e9e7,
                            0x7d3430,
                            cx.listener(SettingsWindow::erase_adi_backend_provisioning),
                        ))
                    },
                )
                .when(
                    matches!(
                        &backend.provisioning_state,
                        AdiProvisioningState::NotProvisioned | AdiProvisioningState::Error(_)
                    ),
                    |this| {
                        this.child(adi_provision_button(
                            provisioning_operation,
                            spinner_turns,
                            cx,
                        ))
                    },
                )
                .when(backend.repair_action.is_some(), |this| {
                    let label = backend
                        .repair_action
                        .map(|action| action.label())
                        .unwrap_or_else(|| "Fix Issues".into());
                    this.child(adi_repair_button(
                        label,
                        color,
                        coreadi_operation,
                        spinner_turns,
                        cx,
                    ))
                    .when(backend.kind == AdiBackendKind::AndroidCoreAdi, |this| {
                        this.child(android_coreadi_apk_link(adi_operation.is_some(), cx))
                    })
                }),
        )
}

fn adi_provision_button(
    provisioning_operation: Option<&AdiOperationState>,
    spinner_turns: f32,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    let provisioning = provisioning_operation.is_some();
    let progress = provisioning_operation
        .map(AdiOperationState::progress)
        .unwrap_or(0.);
    let turn = if provisioning_operation.is_some_and(AdiOperationState::is_indeterminate) {
        spinner_turns
    } else {
        0.
    };
    let label = provisioning_operation
        .map(AdiOperationState::label)
        .unwrap_or_else(|| "Provision device".into());

    div()
        .id("provision-adi")
        .h_8()
        .px_3()
        .rounded_md()
        .bg(rgb(0x173f45))
        .text_color(rgb(0xffffff))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .when(!provisioning, |this| {
            this.cursor_pointer()
                .hover(|style| style.opacity(0.88))
                .on_click(cx.listener(SettingsWindow::provision_adi_backend))
        })
        .when_some(provisioning_operation, |this, _| {
            this.child(progress_circle(progress, turn))
        })
        .child(label)
}

fn adi_repair_button(
    label: String,
    color: u32,
    coreadi_install: Option<&AdiOperationState>,
    spinner_turns: f32,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    let installing = coreadi_install.is_some();
    let progress_label = coreadi_install
        .map(AdiOperationState::label)
        .unwrap_or_else(|| label.into());
    let progress = coreadi_install
        .map(AdiOperationState::progress)
        .unwrap_or(0.);
    let turn = if coreadi_install.is_some_and(AdiOperationState::is_indeterminate) {
        spinner_turns
    } else {
        0.
    };

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
        .gap_2()
        .when(!installing, |this| {
            this.cursor_pointer()
                .hover(|style| style.opacity(0.88))
                .on_click(cx.listener(SettingsWindow::repair_adi_backend))
        })
        .when_some(coreadi_install, |this, _| {
            this.child(progress_circle(progress, turn))
        })
        .child(progress_label)
}

fn adi_backend_detail_row(label: String, value: String) -> gpui::Div {
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .w(px(112.))
                .flex_none()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0x66767c))
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_xs()
                .line_height(px(17.))
                .text_color(rgb(0x53666d))
                .text_ellipsis()
                .child(value),
        )
}

fn adi_status_button(
    id: &'static str,
    label: &'static str,
    disabled: bool,
    bg: u32,
    color: u32,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h_8()
        .px_3()
        .rounded_md()
        .bg(rgb(bg))
        .text_color(rgb(color))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .when(disabled, |this| this.opacity(0.45))
        .when(!disabled, |this| {
            this.cursor_pointer()
                .hover(|style| style.opacity(0.88))
                .on_click(listener)
        })
        .child(label)
}

fn android_coreadi_apk_link(disabled: bool, cx: &mut Context<SettingsWindow>) -> gpui::Div {
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .text_xs()
        .text_color(rgb(0x6a7a81))
        .child("or")
        .child(
            div()
                .id("select-coreadi-apk")
                .text_color(rgb(0x0f6f7a))
                .font_weight(FontWeight::SEMIBOLD)
                .when(disabled, |this| this.opacity(0.45))
                .when(!disabled, |this| {
                    this.cursor_pointer()
                        .hover(|style| style.text_color(rgb(0x173f45)))
                        .on_click(cx.listener(SettingsWindow::select_coreadi_apk))
                })
                .child("Select APK from the disk..."),
        )
}

fn machine_identity_section(
    identity: &MachineIdentity,
    editable: bool,
    editing: Option<&MachineIdentityEdit>,
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
            MachineIdentityField::MachineModel,
            identity.machine_name.clone(),
            editable,
            editing,
            cx,
        ))
        .child(machine_identity_row(
            MachineIdentityField::OsName,
            identity.os_name.clone(),
            editable,
            editing,
            cx,
        ))
        .child(machine_identity_row(
            MachineIdentityField::OsVersion,
            identity.os_version.clone(),
            editable,
            editing,
            cx,
        ))
        .child(machine_identity_row(
            MachineIdentityField::MachineId,
            identity.machine_id.clone(),
            editable,
            editing,
            cx,
        ))
}

fn machine_identity_row(
    field: MachineIdentityField,
    value: SharedString,
    editable: bool,
    editing: Option<&MachineIdentityEdit>,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let label = field.label();
    let is_editing = editing.is_some_and(|edit| edit.field == field);

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
        .when_some(editing.filter(|edit| edit.field == field), |this, edit| {
            this.child(machine_identity_editor(&edit.input))
                .child(machine_identity_icon_button(
                    ("save-machine-identity", machine_identity_id(field)),
                    "icons/check.svg",
                    cx.listener(SettingsWindow::save_machine_identity_edit_from_button),
                ))
                .child(machine_identity_icon_button(
                    ("cancel-machine-identity", machine_identity_id(field)),
                    "icons/x.svg",
                    cx.listener(SettingsWindow::cancel_machine_identity_edit),
                ))
        })
        .when(!is_editing, |this| {
            this.child(
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
                        .id(("edit-machine-identity", machine_identity_id(field)))
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
                        .on_click(cx.listener(move |settings, event, window, cx| {
                            settings.begin_machine_identity_edit(field, event, window, cx)
                        }))
                        .child(lucide_icon("icons/pencil.svg")),
                )
            })
        })
}

fn machine_identity_icon_button(
    id: (&'static str, usize),
    icon: &'static str,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
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
        .on_click(listener)
        .child(lucide_icon(icon))
}

fn machine_identity_id(field: MachineIdentityField) -> usize {
    match field {
        MachineIdentityField::MachineModel => 0,
        MachineIdentityField::OsName => 1,
        MachineIdentityField::OsVersion => 2,
        MachineIdentityField::MachineId => 3,
    }
}

fn status_badge(label: String, selected: bool) -> gpui::Div {
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

fn status_dot(status: &str) -> gpui::Div {
    div()
        .flex_none()
        .w_2()
        .h_2()
        .rounded_full()
        .bg(rgb(backend_status_color(status)))
}

fn backend_status_color(status: &str) -> u32 {
    match status {
        "Ready" => 0x1d6b45,
        "Needs setup" => 0x9a6a14,
        "Offline" => 0x9a302b,
        "Provisioned" => 0x1d6b45,
        "Provisioning" => 0x1d6b45,
        "Not provisioned" => 0x9a6a14,
        "Not available" => 0x53666d,
        "Failed" => 0x9a302b,
        _ => 0x53666d,
    }
}

fn status_badge_bg(status: &str) -> u32 {
    match status {
        "Ready" => 0xe5f3ec,
        "Needs setup" => 0xf6edda,
        "Offline" => 0xf4e4e2,
        "Provisioned" => 0xe5f3ec,
        "Provisioning" => 0xe5f3ec,
        "Not provisioned" => 0xf6edda,
        "Not available" => 0xebf1f0,
        "Failed" => 0xf4e4e2,
        _ => 0xebf1f0,
    }
}

fn status_badge_selected_bg(status: &str) -> u32 {
    match status {
        "Ready" => 0xd3eadc,
        "Needs setup" => 0xeadbb8,
        "Offline" => 0xebcecb,
        "Provisioned" => 0xd3eadc,
        "Provisioning" => 0xd3eadc,
        "Not provisioned" => 0xeadbb8,
        "Not available" => 0xdfe8e6,
        "Failed" => 0xebcecb,
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
