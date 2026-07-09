use super::*;
use gpui_component::{checkbox::Checkbox, Sizable as _};

pub(super) struct AppViewProps<'a> {
    pub(super) focus_handle: &'a FocusHandle,
    pub(super) scroll_handle: &'a ScrollHandle,
    pub(super) app: &'a AppOption,
    pub(super) enabled_patches: &'a [bool],
    pub(super) team_id: &'a str,
    pub(super) app_detail_edit: Option<&'a AppDetailEdit>,
    pub(super) selected_entitlement: Option<usize>,
    pub(super) entitlement_edit: Option<&'a EntitlementEdit>,
    pub(super) entitlement_type_picker_open: bool,
    pub(super) operation_error: Option<SharedString>,
}

pub(super) fn render(props: AppViewProps<'_>, cx: &mut Context<SettingsWindow>) -> gpui::Div {
    let AppViewProps {
        focus_handle,
        scroll_handle,
        app,
        enabled_patches,
        team_id,
        app_detail_edit,
        selected_entitlement,
        entitlement_edit,
        entitlement_type_picker_open,
        operation_error,
    } = props;
    let patch_summary = enabled_patch_summary(app, enabled_patches);

    settings_window_shell()
        .track_focus(focus_handle)
        .capture_key_down(cx.listener(SettingsWindow::handle_app_settings_key))
        .child(scroll_panel(
            "app-settings-scroll",
            scroll_handle,
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(settings_window_header(
                    "App Settings",
                    "Review metadata and entitlements before signing.",
                ))
                .when_some(operation_error, |this, error| {
                    this.child(app_settings_error(error))
                })
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
                                    AppMetadataField::Name,
                                    app,
                                    true,
                                    app_detail_edit,
                                    cx,
                                ))
                                .child(app_detail_row_with_display_value(
                                    AppMetadataField::BundleId,
                                    app,
                                    true,
                                    app_detail_edit,
                                    crate::app::entitlements::effective_bundle_identifier_for_app(
                                        app, team_id,
                                    ),
                                    cx,
                                ))
                                .child(app_detail_row(
                                    AppMetadataField::Version,
                                    app,
                                    true,
                                    app_detail_edit,
                                    cx,
                                ))
                                .child(app_detail_row(
                                    AppMetadataField::Build,
                                    app,
                                    true,
                                    app_detail_edit,
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
                        .child(app_detail_row(
                            AppMetadataField::Executable,
                            app,
                            true,
                            app_detail_edit,
                            cx,
                        ))
                        .child(app_detail_row(
                            AppMetadataField::MinimumOs,
                            app,
                            true,
                            app_detail_edit,
                            cx,
                        ))
                        .child(app_detail_row(
                            AppMetadataField::SupportedDevices,
                            app,
                            true,
                            app_detail_edit,
                            cx,
                        ))
                        .child(read_only_app_detail_row("IPA", app.path.to_string()))
                        .child(read_only_app_detail_row("Patches", patch_summary)),
                )
                .child(entitlements_table(
                    app,
                    team_id,
                    selected_entitlement,
                    entitlement_edit,
                    entitlement_type_picker_open,
                    cx,
                )),
        ))
}

fn app_settings_error(error: SharedString) -> impl IntoElement {
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

fn app_icon_editor(app: &AppOption, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    let initial = app
        .name()
        .to_string()
        .chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "A".to_string());
    let icon_path = app.displayed_icon_path().map(|path| path.to_string());

    surface_button("app-icon-editor")
        .flex_none()
        .w(px(84.))
        .h(px(84.))
        .cursor_pointer()
        .on_click(cx.listener(SettingsWindow::edit_app_icon))
        .child(
            div()
                .size_full()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xcfd8d6))
                .bg(fixed_rgb(0x173f45))
                .text_color(fixed_rgb(0xffffff))
                .overflow_hidden()
                .hover(|style| style.bg(fixed_rgb(0x20545c)))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_1()
                .child(if let Some(icon_path) = icon_path {
                    div().size_full().overflow_hidden().child(
                        img(Arc::<std::path::Path>::from(std::path::PathBuf::from(
                            icon_path,
                        )))
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                    )
                } else {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_1()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child(initial)
                        .child(div().text_xs().child("Edit icon"))
                }),
        )
}

fn app_detail_row(
    field: AppMetadataField,
    app: &AppOption,
    editable: bool,
    edit: Option<&AppDetailEdit>,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    app_detail_row_inner(field, app, editable, edit, None, cx)
}

fn app_detail_row_with_display_value(
    field: AppMetadataField,
    app: &AppOption,
    editable: bool,
    edit: Option<&AppDetailEdit>,
    display_value: String,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    app_detail_row_inner(field, app, editable, edit, Some(display_value), cx)
}

fn app_detail_row_inner(
    field: AppMetadataField,
    app: &AppOption,
    editable: bool,
    edit: Option<&AppDetailEdit>,
    display_value: Option<String>,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let is_editing = edit.is_some_and(|edit| edit.field() == field);
    let changed = app.field_is_overridden(field);
    let effective_changed = display_value
        .as_deref()
        .is_some_and(|value| value != app.field_display_value(field));

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
                .child(field.label()),
        )
        .child(
            if let Some(AppDetailEdit::Text { input, .. }) =
                edit.filter(|edit| edit.field() == field)
            {
                machine_identity_editor(input)
            } else if let Some(AppDetailEdit::SupportedDevices { selected }) =
                edit.filter(|edit| edit.field() == field)
            {
                supported_devices_editor(selected, cx)
            } else {
                div()
                    .min_w_0()
                    .flex_1()
                    .text_sm()
                    .text_color(rgb(if changed || effective_changed {
                        0x173f45
                    } else {
                        0x405057
                    }))
                    .text_ellipsis()
                    .child(display_value.unwrap_or_else(|| app.field_display_value(field)))
            },
        )
        .when(editable, |this| {
            this.child(app_detail_controls(field, changed, is_editing, cx))
        })
}

fn supported_devices_editor(
    selected: &[SupportedDeviceFamily],
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_wrap()
        .gap_1()
        .children(SupportedDeviceFamily::ALL.into_iter().map(|family| {
            let active = selected.contains(&family);
            surface_button(("supported-device-family", family as usize))
                .h_6()
                .cursor_pointer()
                .on_click(cx.listener(move |settings, event, window, cx| {
                    settings.toggle_supported_device_family(family, event, window, cx)
                }))
                .child(
                    div()
                        .size_full()
                        .px_2()
                        .rounded_md()
                        .border_1()
                        .border_color(if active { rgb(0x168291) } else { rgb(0xd8e0df) })
                        .bg(if active { rgb(0xf0fbfa) } else { rgb(0xffffff) })
                        .text_color(rgb(if active { 0x173f45 } else { 0x53666d }))
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|style| style.bg(rgb(0xeef6f5)))
                        .child(family.label()),
                )
        }))
}

fn read_only_app_detail_row(label: &'static str, value: String) -> gpui::Div {
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
}

fn app_detail_controls(
    field: AppMetadataField,
    changed: bool,
    editing: bool,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    div()
        .flex_none()
        .w(px(86.))
        .flex()
        .items_center()
        .justify_end()
        .gap_1()
        .child(app_detail_revert_slot(field, changed, cx))
        .when(editing, |this| {
            this.child(confirm_app_detail_button(cx))
                .child(cancel_app_detail_button(cx))
        })
        .when(!editing, |this| this.child(edit_button(field, cx)))
}

fn edit_button(field: AppMetadataField, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    surface_button(("edit-app-detail", app_detail_id(field)))
        .flex_none()
        .w_6()
        .h_6()
        .cursor_pointer()
        .on_click(cx.listener(move |settings, event, window, cx| {
            settings.begin_app_detail_edit(field, event, window, cx)
        }))
        .child(icon_button_surface(
            "icons/pencil.svg",
            0xebf1f0,
            0xdfe8e6,
            0x53666d,
        ))
}

fn confirm_app_detail_button(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    surface_button("save-app-detail")
        .flex_none()
        .w_6()
        .h_6()
        .cursor_pointer()
        .on_click(cx.listener(SettingsWindow::save_app_detail_edit_from_button))
        .child(icon_button_surface(
            "icons/check.svg",
            0xd3eadc,
            0xc5e2d1,
            0x1d6b45,
        ))
}

fn cancel_app_detail_button(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    surface_button("cancel-app-detail")
        .flex_none()
        .w_6()
        .h_6()
        .cursor_pointer()
        .on_click(cx.listener(SettingsWindow::cancel_app_detail_edit))
        .child(icon_button_surface(
            "icons/x.svg",
            0xebcecb,
            0xe2c0bd,
            0x9a302b,
        ))
}

fn app_detail_revert_slot(
    field: AppMetadataField,
    changed: bool,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    surface_button(("revert-app-detail", app_detail_id(field)))
        .flex_none()
        .w_6()
        .h_6()
        .when(!changed, |this| this.opacity(0.).tab_stop(false))
        .when(changed, |this| {
            this.cursor_pointer()
                .on_click(cx.listener(move |settings, event, window, cx| {
                    settings.revert_app_detail_field(field, event, window, cx)
                }))
        })
        .child(icon_button_surface(
            "icons/rotate-ccw.svg",
            0xebf1f0,
            0xdfe8e6,
            0x53666d,
        ))
}

fn app_detail_id(field: AppMetadataField) -> usize {
    match field {
        AppMetadataField::Name => 0,
        AppMetadataField::BundleId => 1,
        AppMetadataField::Version => 2,
        AppMetadataField::Build => 3,
        AppMetadataField::Executable => 4,
        AppMetadataField::MinimumOs => 5,
        AppMetadataField::SupportedDevices => 6,
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

fn entitlements_table(
    app: &AppOption,
    team_id: &str,
    selected_entitlement: Option<usize>,
    edit: Option<&EntitlementEdit>,
    type_picker_open: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let changed = app.entitlements_are_overridden();
    let entitlements = crate::app::entitlements::effective_entitlements_for_app(app, team_id);
    let default_entitlements =
        crate::app::entitlements::default_effective_entitlements_for_app(app, team_id);

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
                .child(entitlements_actions(changed, cx)),
        )
        .child(entitlement_header_row())
        .children(entitlements.iter().enumerate().map(|(index, entitlement)| {
            let row_changed = changed
                && entitlement_revert_target(entitlement, index, &default_entitlements).as_ref()
                    != Some(entitlement);
            entitlement_row(
                index,
                entitlement,
                selected_entitlement == Some(index),
                row_changed,
                edit,
                type_picker_open,
                cx,
            )
        }))
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
    surface_button(id)
        .flex_none()
        .w_6()
        .h_6()
        .cursor_pointer()
        .on_click(listener)
        .child(icon_button_surface(icon, 0xebf1f0, 0xdfe8e6, 0x53666d))
}

fn entitlements_revert_slot(changed: bool, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    surface_button("revert-entitlements")
        .flex_none()
        .w_6()
        .h_6()
        .when(!changed, |this| this.opacity(0.).tab_stop(false))
        .when(changed, |this| {
            this.cursor_pointer()
                .on_click(cx.listener(SettingsWindow::revert_entitlements))
        })
        .child(icon_button_surface(
            "icons/rotate-ccw.svg",
            0xebf1f0,
            0xdfe8e6,
            0x53666d,
        ))
}

fn entitlement_header_row() -> gpui::Div {
    div()
        .flex()
        .items_center()
        .min_h(px(36.))
        .px_3()
        .bg(rgb(0xf6f9f9))
        .border_b_1()
        .border_color(rgb(0xd8e0df))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x66767c))
        .child(entitlement_column(EntitlementField::Key, "Key"))
        .child(entitlement_column(EntitlementField::ValueType, "Type"))
        .child(entitlement_column(EntitlementField::Value, "Value"))
}

fn entitlement_row(
    index: usize,
    entitlement: &AppEntitlement,
    selected: bool,
    changed: bool,
    edit: Option<&EntitlementEdit>,
    type_picker_open: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .min_h(px(38.))
        .px_3()
        .py_1()
        .border_b_1()
        .border_color(rgb(0xe8eeee))
        .text_xs()
        .text_color(rgb(0x405057))
        .bg(if selected {
            rgb(0xf0fbfa)
        } else {
            rgb(0xffffff)
        })
        .child(entitlement_column(
            EntitlementField::Key,
            entitlement_cell(
                index,
                EntitlementField::Key,
                entitlement,
                edit,
                type_picker_open,
                false,
                cx,
            ),
        ))
        .child(entitlement_column(
            EntitlementField::ValueType,
            entitlement_cell(
                index,
                EntitlementField::ValueType,
                entitlement,
                edit,
                type_picker_open,
                false,
                cx,
            ),
        ))
        .child(entitlement_column(
            EntitlementField::Value,
            entitlement_cell(
                index,
                EntitlementField::Value,
                entitlement,
                edit,
                type_picker_open,
                changed,
                cx,
            ),
        ))
}

fn entitlement_column(field: EntitlementField, child: impl IntoElement) -> gpui::Div {
    div()
        .min_w_0()
        .when(field == EntitlementField::Key, |this| {
            this.w(px(260.)).flex_none()
        })
        .when(field == EntitlementField::ValueType, |this| {
            this.w(px(136.)).flex_none()
        })
        .when(field == EntitlementField::Value, |this| this.flex_1())
        .child(child)
}

fn entitlement_cell(
    row: usize,
    field: EntitlementField,
    entitlement: &AppEntitlement,
    edit: Option<&EntitlementEdit>,
    type_picker_open: bool,
    row_revert_visible: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::AnyElement {
    let is_editing = edit.is_some_and(|edit| edit.row() == row && edit.field() == field);
    let has_row_revert = row_revert_visible && field == EntitlementField::Value && !is_editing;
    let value = if let Some(EntitlementEdit::Text { input, .. }) =
        edit.filter(|edit| edit.row() == row && edit.field() == field)
    {
        machine_identity_editor(input).into_any_element()
    } else if let Some(EntitlementEdit::Type { selected, .. }) =
        edit.filter(|edit| edit.row() == row && edit.field() == field)
    {
        entitlement_type_editor(selected, type_picker_open, cx).into_any_element()
    } else if let Some(EntitlementEdit::Array { items, .. }) =
        edit.filter(|edit| edit.row() == row && edit.field() == field)
    {
        array_entitlement_editor(items, cx).into_any_element()
    } else if let Some(EntitlementEdit::Boolean { value, .. }) =
        edit.filter(|edit| edit.row() == row && edit.field() == field)
    {
        boolean_entitlement_editor(*value, cx).into_any_element()
    } else {
        div()
            .min_w_0()
            .flex_1()
            .text_xs()
            .line_height(px(18.))
            .text_ellipsis()
            .child(entitlement_field_value(entitlement, field).clone())
            .into_any_element()
    };

    let body = div()
        .min_w_0()
        .w_full()
        .h_7()
        .flex()
        .items_center()
        .gap_1()
        .rounded_sm()
        .text_xs()
        .line_height(px(18.))
        .child(value)
        .when(has_row_revert, |this| {
            this.child(entitlement_row_revert_button(row, cx))
        })
        .when(is_editing, |this| {
            this.child(entitlement_edit_button(
                ("save-entitlement", row * 3 + entitlement_field_id(field)),
                "icons/check.svg",
                cx.listener(SettingsWindow::save_entitlement_edit_from_button),
            ))
            .child(entitlement_edit_button(
                ("cancel-entitlement", row * 3 + entitlement_field_id(field)),
                "icons/x.svg",
                cx.listener(SettingsWindow::cancel_entitlement_edit),
            ))
        });

    if is_editing || has_row_revert {
        body.id(("entitlement-cell", row * 3 + entitlement_field_id(field)))
            .into_any_element()
    } else {
        surface_button(("entitlement-cell", row * 3 + entitlement_field_id(field)))
            .min_w_0()
            .w_full()
            .h_7()
            .text_xs()
            .line_height(px(18.))
            .cursor_pointer()
            .on_click(cx.listener(move |settings, event, window, cx| {
                settings.begin_entitlement_edit(row, field, event, window, cx)
            }))
            .child(body.hover(|style| style.bg(rgb(0xeef6f5))))
            .into_any_element()
    }
}

fn entitlement_row_revert_button(row: usize, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    surface_button(("revert-entitlement", row))
        .flex_none()
        .w_6()
        .h_6()
        .cursor_pointer()
        .on_click(cx.listener(move |settings, event, window, cx| {
            settings.revert_entitlement(row, event, window, cx)
        }))
        .child(icon_button_surface(
            "icons/rotate-ccw.svg",
            0xebf1f0,
            0xdfe8e6,
            0x53666d,
        ))
}

fn boolean_entitlement_editor(value: bool, cx: &mut Context<SettingsWindow>) -> gpui::Div {
    div().min_w_0().flex_1().child(
        Checkbox::new("boolean-entitlement-editor")
            .small()
            .checked(value)
            .on_click(cx.listener(|settings, checked: &bool, _, cx| {
                if let Some(EntitlementEdit::Boolean { value, .. }) = &mut settings.entitlement_edit
                {
                    *value = *checked;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .min_w_0()
                    .text_xs()
                    .text_color(rgb(0x405057))
                    .child(if value { "true" } else { "false" }),
            ),
    )
}

fn array_entitlement_editor(
    items: &[Entity<InputState>],
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .gap_1()
        .children(items.iter().enumerate().map(|(index, input)| {
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap_1()
                .child(machine_identity_editor(input))
                .child(array_entitlement_item_button(
                    ("remove-entitlement-array-item", index),
                    "icons/minus.svg",
                    cx.listener(move |settings, event, window, cx| {
                        settings.remove_entitlement_array_item(index, event, window, cx)
                    }),
                ))
        }))
        .child(
            surface_button("add-entitlement-array-item")
                .h_6()
                .w(px(92.))
                .cursor_pointer()
                .on_click(cx.listener(SettingsWindow::add_entitlement_array_item))
                .child(action_button_surface(
                    "Add item",
                    0xebf1f0,
                    0xdfe8e6,
                    0x53666d,
                    Some(lucide_icon_tinted("icons/plus.svg", 0x53666d)),
                )),
        )
}

fn array_entitlement_item_button(
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

fn entitlement_type_editor(
    selected: &str,
    open: bool,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    let trigger = surface_button("entitlement-type-select")
        .h_7()
        .w_full()
        .cursor_pointer()
        .child(
            div()
                .size_full()
                .px_2()
                .rounded_md()
                .border_1()
                .border_color(if open { rgb(0x0f6f7a) } else { rgb(0xcfd8d6) })
                .bg(rgb(0xffffff))
                .hover(|style| style.bg(rgb(0xeef6f5)))
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_ellipsis()
                        .text_xs()
                        .text_color(rgb(0x405057))
                        .child(selected.to_string()),
                )
                .child(chevron(open)),
        );

    floating_select_popover(
        "entitlement-type-popover-scroll",
        trigger,
        open,
        cx.listener(|settings, open: &bool, _, cx| {
            if matches!(
                settings.entitlement_edit,
                Some(EntitlementEdit::Type { .. })
            ) {
                settings.entitlement_type_picker_open = *open;
            } else {
                settings.entitlement_type_picker_open = false;
            }
            cx.notify();
        }),
        FloatingPopoverLayout {
            width: 160.,
            max_height: 220.,
            offset_y: 34.,
        },
        dropdown_list(
            EntitlementValue::EDITABLE_TYPE_LABELS
                .into_iter()
                .enumerate()
                .map(|(index, label)| entitlement_type_option(index, label, label == selected, cx)),
        ),
    )
}

fn entitlement_type_option(
    index: usize,
    label: &'static str,
    selected: bool,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    surface_button(("entitlement-type-option", index))
        .h_7()
        .w_full()
        .cursor_pointer()
        .on_click(cx.listener(move |settings, event, window, cx| {
            settings.select_entitlement_type(label, event, window, cx)
        }))
        .child(
            div()
                .size_full()
                .px_2()
                .rounded_sm()
                .bg(if selected {
                    rgb(0xf0fbfa)
                } else {
                    rgb(0xffffff)
                })
                .text_color(rgb(if selected { 0x173f45 } else { 0x405057 }))
                .text_xs()
                .font_weight(if selected {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .flex()
                .items_center()
                .hover(|style| style.bg(rgb(0xeef6f5)))
                .child(div().text_xs().child(label)),
        )
}

fn entitlement_field_id(field: EntitlementField) -> usize {
    match field {
        EntitlementField::Key => 0,
        EntitlementField::ValueType => 1,
        EntitlementField::Value => 2,
    }
}

fn entitlement_edit_button(
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
