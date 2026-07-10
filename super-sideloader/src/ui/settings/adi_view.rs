use super::*;
use crate::app::models::AdiBackendAvailability;

pub(super) struct AdiViewProps<'a> {
    pub(super) focus_handle: &'a FocusHandle,
    pub(super) scroll_handle: &'a ScrollHandle,
    pub(super) theme_preference: ThemePreference,
    pub(super) backends: &'a [AdiBackendOption],
    pub(super) selected_backend: usize,
    pub(super) machine_identity: &'a MachineIdentity,
    pub(super) android_device_identity: &'a MachineIdentity,
    pub(super) theme_picker_open: bool,
    pub(super) adi_backend_picker_open: bool,
    pub(super) adi_operation: Option<&'a AdiOperationState>,
    pub(super) spinner_turns: f32,
    pub(super) machine_identity_edit: Option<&'a MachineIdentityEdit>,
}

pub(super) fn render(props: AdiViewProps<'_>, cx: &mut Context<SettingsWindow>) -> gpui::Div {
    let AdiViewProps {
        focus_handle,
        scroll_handle,
        theme_preference,
        backends,
        selected_backend,
        machine_identity,
        android_device_identity,
        theme_picker_open,
        adi_backend_picker_open,
        adi_operation,
        spinner_turns,
        machine_identity_edit,
    } = props;
    let selected = selected_backend.min(backends.len().saturating_sub(1));
    let backend = backends.get(selected);
    let displayed_machine_identity = backend
        .map(|backend| {
            SideloaderState::machine_identity_for_adi_backend(
                backend.kind,
                machine_identity,
                android_device_identity,
            )
        })
        .unwrap_or(machine_identity);
    let editable_identity = backend
        .map(|backend| backend.editable_identity)
        .unwrap_or(false);
    let backend_controls_disabled = adi_operation.is_some();

    let mut content = div()
        .flex()
        .flex_col()
        .gap_4()
        .child(appearance_section(theme_preference, theme_picker_open, cx))
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
                    let trigger =
                        settings_adi_backend_select_button(backend, adi_backend_picker_open)
                            .when(backend_controls_disabled, |this| {
                                this.opacity(0.55).tab_stop(false)
                            });
                    this.child(if backend_controls_disabled {
                        trigger.into_any_element()
                    } else {
                        settings_backend_select_with_popover(
                            trigger,
                            adi_backend_picker_open,
                            cx.listener(|this, open: &bool, _, cx| {
                                if this.adi_operation.is_some() {
                                    this.adi_backend_picker_open = false;
                                } else if matches!(this.mode, SettingsMode::AdiSettings) {
                                    this.adi_backend_picker_open = *open;
                                }
                                cx.notify();
                            }),
                            dropdown_list((0..backends.len()).map(|index| {
                                settings_adi_backend_option(
                                    &backends[index],
                                    index,
                                    index == selected,
                                )
                                .on_click(cx.listener(
                                    move |this, event, window, cx| {
                                        this.select_adi_backend(index, event, window, cx)
                                    },
                                ))
                            }))
                            .id("adi-backend-options"),
                        )
                        .into_any_element()
                    })
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

fn appearance_section(
    theme_preference: ThemePreference,
    theme_picker_open: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0x66767c))
                .child("Appearance"),
        )
        .child(settings_theme_select_with_popover(
            settings_theme_select_button(theme_preference, theme_picker_open),
            theme_picker_open,
            cx.listener(|this, open: &bool, _, cx| {
                if matches!(this.mode, SettingsMode::AdiSettings) {
                    this.theme_picker_open = *open;
                    if *open {
                        this.adi_backend_picker_open = false;
                    }
                }
                cx.notify();
            }),
            dropdown_list(ThemePreference::options().into_iter().enumerate().map(
                |(index, preference)| {
                    settings_theme_option(preference, index, preference == theme_preference)
                        .on_click(cx.listener(move |this, event, window, cx| {
                            this.select_theme_preference(preference, event, window, cx)
                        }))
                },
            ))
            .id("theme-options"),
        ))
}

fn settings_theme_select_with_popover(
    trigger: Button,
    open: bool,
    on_open_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    popover: impl IntoElement,
) -> impl IntoElement {
    floating_select_popover(
        "theme-popover-scroll",
        trigger,
        open,
        on_open_change,
        FloatingPopoverLayout {
            width: ADI_BACKEND_POPOVER_WIDTH,
            max_height: ADI_BACKEND_POPOVER_MAX_HEIGHT,
            offset_y: ADI_BACKEND_COMBO_HEIGHT + 8.,
        },
        popover,
    )
}

fn settings_theme_select_button(preference: ThemePreference, open: bool) -> Button {
    surface_button("theme-select")
        .h(px(ADI_BACKEND_COMBO_HEIGHT))
        .w_full()
        .cursor_pointer()
        .child(
            div()
                .min_w_0()
                .size_full()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(if open { rgb(0x0f6f7a) } else { rgb(0xcfd8d6) })
                .bg(rgb(0xffffff))
                .hover(|style| style.bg(rgb(0xeef6f5)))
                .flex()
                .items_center()
                .gap_3()
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
                                .child(preference.label()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x6a7a81))
                                .text_ellipsis()
                                .child(preference.detail()),
                        ),
                )
                .child(chevron(open)),
        )
}

fn settings_theme_option(preference: ThemePreference, index: usize, selected: bool) -> Button {
    surface_button(("theme-option", index))
        .min_h(px(56.))
        .w_full()
        .cursor_pointer()
        .child(
            div()
                .min_w_0()
                .size_full()
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
                .hover(|style| style.bg(rgb(0xeef6f5)))
                .flex()
                .items_center()
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
                                .child(preference.label()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x6a7a81))
                                .text_ellipsis()
                                .child(preference.detail()),
                        ),
                ),
        )
}

fn settings_backend_select_with_popover(
    trigger: Button,
    open: bool,
    on_open_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    popover: impl IntoElement,
) -> impl IntoElement {
    floating_select_popover(
        "adi-backend-popover-scroll",
        trigger,
        open,
        on_open_change,
        FloatingPopoverLayout {
            width: ADI_BACKEND_POPOVER_WIDTH,
            max_height: ADI_BACKEND_POPOVER_MAX_HEIGHT,
            offset_y: ADI_BACKEND_COMBO_HEIGHT + 8.,
        },
        popover,
    )
}

fn settings_adi_backend_select_button(backend: &AdiBackendOption, open: bool) -> Button {
    let status = backend.availability.label();

    surface_button("adi-backend-select")
        .h(px(ADI_BACKEND_COMBO_HEIGHT))
        .w_full()
        .cursor_pointer()
        .child(
            div()
                .min_w_0()
                .size_full()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(if open { rgb(0x0f6f7a) } else { rgb(0xcfd8d6) })
                .bg(rgb(0xffffff))
                .hover(|style| style.bg(rgb(0xeef6f5)))
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
                .child(chevron(open)),
        )
}

fn settings_adi_backend_option(backend: &AdiBackendOption, index: usize, selected: bool) -> Button {
    let status = backend.availability.label();

    surface_button(("adi-backend-option", index))
        .min_h(px(56.))
        .w_full()
        .cursor_pointer()
        .child(
            div()
                .min_w_0()
                .size_full()
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
                .hover(|style| style.bg(rgb(0xeef6f5)))
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
                .child(status_badge(status, selected)),
        )
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
                    should_show_provision_action(backend.availability, &backend.provisioning_state),
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

fn should_show_provision_action(
    availability: AdiBackendAvailability,
    provisioning_state: &AdiProvisioningState,
) -> bool {
    availability.is_ready()
        && matches!(
            provisioning_state,
            AdiProvisioningState::NotProvisioned | AdiProvisioningState::Error(_)
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

    surface_button("provision-adi")
        .h_8()
        .w_full()
        .when(provisioning, |this| this.tab_stop(false))
        .when(!provisioning, |this| {
            this.cursor_pointer()
                .on_click(cx.listener(SettingsWindow::provision_adi_backend))
        })
        .child(primary_action_button_surface(
            label,
            provisioning_operation.map(|_| progress_circle(progress, turn)),
        ))
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

    surface_button("repair-adi-backend")
        .h_8()
        .w_full()
        .when(installing, |this| this.tab_stop(false))
        .when(!installing, |this| {
            this.cursor_pointer()
                .on_click(cx.listener(SettingsWindow::repair_adi_backend))
        })
        .child(action_button_surface(
            progress_label,
            color,
            color,
            0xffffff,
            coreadi_install.map(|_| progress_circle(progress, turn)),
        ))
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
    surface_button(id)
        .h_8()
        .when(disabled, |this| this.opacity(0.45).tab_stop(false))
        .when(!disabled, |this| this.cursor_pointer().on_click(listener))
        .child(action_button_surface(
            label,
            bg,
            0xfff7f6,
            color,
            None::<gpui::Div>,
        ))
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
            surface_button("select-coreadi-apk")
                .text_color(rgb(0x0f6f7a))
                .font_weight(FontWeight::SEMIBOLD)
                .when(disabled, |this| this.opacity(0.45).tab_stop(false))
                .when(!disabled, |this| {
                    this.cursor_pointer()
                        .on_click(cx.listener(SettingsWindow::select_coreadi_apk))
                })
                .child(
                    div()
                        .size_full()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x0f6f7a))
                        .when(!disabled, |this| {
                            this.hover(|style| style.text_color(rgb(0x173f45)))
                        })
                        .flex()
                        .items_center()
                        .child("Select APK from the disk..."),
                ),
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
    value: String,
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
                    surface_button(("edit-machine-identity", machine_identity_id(field)))
                        .flex_none()
                        .w_6()
                        .h_6()
                        .cursor_pointer()
                        .on_click(cx.listener(move |settings, event, window, cx| {
                            settings.begin_machine_identity_edit(field, event, window, cx)
                        }))
                        .child(icon_button_surface(
                            "icons/pencil.svg",
                            0xebf1f0,
                            0xdfe8e6,
                            0x53666d,
                        )),
                )
            })
        })
}

fn machine_identity_icon_button(
    id: (&'static str, usize),
    icon: &'static str,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    surface_button(id)
        .flex_none()
        .w_6()
        .h_6()
        .cursor_pointer()
        .on_click(listener)
        .child(icon_button_surface(icon, 0xebf1f0, 0xdfe8e6, 0x53666d))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provisioning_action_requires_a_ready_backend() {
        let error = AdiProvisioningState::Error("installation failed".to_string());

        assert!(!should_show_provision_action(
            AdiBackendAvailability::NeedsSetup,
            &error
        ));
        assert!(should_show_provision_action(
            AdiBackendAvailability::Ready,
            &error
        ));
        assert!(should_show_provision_action(
            AdiBackendAvailability::Ready,
            &AdiProvisioningState::NotProvisioned
        ));
        assert!(!should_show_provision_action(
            AdiBackendAvailability::Ready,
            &AdiProvisioningState::Provisioned
        ));
    }
}
