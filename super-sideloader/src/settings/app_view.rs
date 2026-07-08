use super::*;

pub(super) fn render(
    focus_handle: &FocusHandle,
    scroll_handle: &ScrollHandle,
    app: &AppOption,
    enabled_patches: &[bool],
    team_id: &SharedString,
    app_detail_edit: Option<&AppDetailEdit>,
    selected_entitlement: Option<usize>,
    entitlement_edit: Option<&EntitlementEdit>,
    entitlement_type_picker_open: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
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
                                .child(app_detail_row(
                                    AppMetadataField::BundleId,
                                    app,
                                    true,
                                    app_detail_edit,
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

fn app_icon_editor(app: &AppOption, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    let initial = app
        .name()
        .to_string()
        .chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "A".to_string());
    let icon_path = app.displayed_icon_path().map(|path| path.to_string());

    div()
        .flex_none()
        .w(px(84.))
        .h(px(84.))
        .rounded_md()
        .border_1()
        .border_color(rgb(0xcfd8d6))
        .bg(rgb(0x173f45))
        .text_color(rgb(0xffffff))
        .overflow_hidden()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x20545c)))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .id("app-icon-editor")
        .on_click(cx.listener(SettingsWindow::edit_app_icon))
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
        })
}

fn app_detail_row(
    field: AppMetadataField,
    app: &AppOption,
    editable: bool,
    edit: Option<&AppDetailEdit>,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let is_editing = edit.is_some_and(|edit| edit.field() == field);
    let changed = app.field_is_overridden(field);

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
                    .text_color(rgb(if changed { 0x173f45 } else { 0x405057 }))
                    .text_ellipsis()
                    .child(app.field_display_value(field))
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
            div()
                .id(("supported-device-family", family as usize))
                .px_2()
                .h_6()
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
                .cursor_pointer()
                .hover(|style| style.bg(rgb(0xf6f9f9)))
                .on_click(cx.listener(move |settings, event, window, cx| {
                    settings.toggle_supported_device_family(family, event, window, cx)
                }))
                .child(family.label())
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
        .id(("edit-app-detail", app_detail_id(field)))
        .on_click(cx.listener(move |settings, event, window, cx| {
            settings.begin_app_detail_edit(field, event, window, cx)
        }))
        .child(lucide_icon("icons/pencil.svg"))
}

fn confirm_app_detail_button(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .flex_none()
        .w_6()
        .h_6()
        .rounded_md()
        .bg(rgb(0xd3eadc))
        .text_color(rgb(0x1d6b45))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xc5e2d1)))
        .id("save-app-detail")
        .on_click(cx.listener(SettingsWindow::save_app_detail_edit_from_button))
        .child(lucide_icon("icons/check.svg"))
}

fn cancel_app_detail_button(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .flex_none()
        .w_6()
        .h_6()
        .rounded_md()
        .bg(rgb(0xebcecb))
        .text_color(rgb(0x9a302b))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xe2c0bd)))
        .id("cancel-app-detail")
        .on_click(cx.listener(SettingsWindow::cancel_app_detail_edit))
        .child(lucide_icon("icons/x.svg"))
}

fn app_detail_revert_slot(
    field: AppMetadataField,
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
        .id(("revert-app-detail", app_detail_id(field)))
        .when(!changed, |this| this.opacity(0.))
        .when(changed, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(rgb(0xdfe8e6)))
                .on_click(cx.listener(move |settings, event, window, cx| {
                    settings.revert_app_detail_field(field, event, window, cx)
                }))
        })
        .child("Revert")
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
    team_id: &SharedString,
    selected_entitlement: Option<usize>,
    edit: Option<&EntitlementEdit>,
    type_picker_open: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let changed = app.entitlements_are_overridden();
    let entitlements = app.effective_entitlements(team_id);
    let default_entitlements = app.default_effective_entitlements(team_id);

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
        .w_6()
        .h_6()
        .rounded_md()
        .bg(rgb(0xebf1f0))
        .text_color(rgb(0x53666d))
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
        .child(lucide_icon("icons/rotate-ccw.svg"))
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
        .grid()
        .grid_cols(3)
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
        .child(entitlement_cell(
            index,
            EntitlementField::Key,
            entitlement,
            edit,
            type_picker_open,
            false,
            cx,
        ))
        .child(entitlement_cell(
            index,
            EntitlementField::ValueType,
            entitlement,
            edit,
            type_picker_open,
            false,
            cx,
        ))
        .child(entitlement_cell(
            index,
            EntitlementField::Value,
            entitlement,
            edit,
            type_picker_open,
            changed,
            cx,
        ))
}

fn entitlement_cell(
    row: usize,
    field: EntitlementField,
    entitlement: &AppEntitlement,
    edit: Option<&EntitlementEdit>,
    type_picker_open: bool,
    row_revert_visible: bool,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    let is_editing = edit.is_some_and(|edit| edit.row() == row && edit.field() == field);
    div()
        .min_w_0()
        .min_h_7()
        .flex()
        .items_center()
        .gap_1()
        .rounded_sm()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xf6f9f9)))
        .id(("entitlement-cell", row * 3 + entitlement_field_id(field)))
        .on_click(cx.listener(move |settings, event, window, cx| {
            settings.begin_entitlement_edit(row, field, event, window, cx)
        }))
        .child(
            if let Some(EntitlementEdit::Text { input, .. }) =
                edit.filter(|edit| edit.row() == row && edit.field() == field)
            {
                machine_identity_editor(input)
            } else if let Some(EntitlementEdit::Type { selected, .. }) =
                edit.filter(|edit| edit.row() == row && edit.field() == field)
            {
                entitlement_type_editor(selected, type_picker_open, cx)
            } else if let Some(EntitlementEdit::Array { items, .. }) =
                edit.filter(|edit| edit.row() == row && edit.field() == field)
            {
                array_entitlement_editor(items, cx)
            } else if let Some(EntitlementEdit::Boolean { value, .. }) =
                edit.filter(|edit| edit.row() == row && edit.field() == field)
            {
                boolean_entitlement_editor(*value, cx)
            } else {
                div()
                    .min_w_0()
                    .flex_1()
                    .text_ellipsis()
                    .child(entitlement_field_value(entitlement, field).clone())
            },
        )
        .when(
            row_revert_visible && field == EntitlementField::Value && !is_editing,
            |this| this.child(entitlement_row_revert_button(row, cx)),
        )
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
        })
}

fn entitlement_row_revert_button(row: usize, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .id(("revert-entitlement", row))
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
            settings.revert_entitlement(row, event, window, cx)
        }))
        .child(lucide_icon("icons/rotate-ccw.svg"))
}

fn boolean_entitlement_editor(value: bool, cx: &mut Context<SettingsWindow>) -> gpui::Div {
    div().min_w_0().flex_1().child(
        div()
            .id("boolean-entitlement-editor")
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .on_click(cx.listener(SettingsWindow::toggle_boolean_entitlement_value))
            .child(
                div()
                    .flex_none()
                    .w_5()
                    .h_5()
                    .rounded_sm()
                    .border_1()
                    .border_color(if value { rgb(0x168291) } else { rgb(0xcfd8d6) })
                    .bg(if value { rgb(0xd3eadc) } else { rgb(0xffffff) })
                    .text_color(rgb(0x1d6b45))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(value, |this| this.child(lucide_icon("icons/check.svg"))),
            )
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
    items: &[Entity<EditLine>],
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
            div()
                .id("add-entitlement-array-item")
                .h_6()
                .px_2()
                .rounded_md()
                .bg(rgb(0xebf1f0))
                .text_color(rgb(0x53666d))
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .flex()
                .items_center()
                .justify_center()
                .gap_1()
                .cursor_pointer()
                .hover(|style| style.bg(rgb(0xdfe8e6)))
                .on_click(cx.listener(SettingsWindow::add_entitlement_array_item))
                .child(lucide_icon("icons/plus.svg"))
                .child("Add item"),
        )
}

fn array_entitlement_item_button(
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

fn entitlement_type_editor(
    selected: &SharedString,
    open: bool,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    div()
        .relative()
        .min_w_0()
        .flex_1()
        .child(
            div()
                .id("entitlement-type-combobox")
                .h_7()
                .px_2()
                .rounded_md()
                .border_1()
                .border_color(if open { rgb(0x0f6f7a) } else { rgb(0xcfd8d6) })
                .bg(rgb(0xffffff))
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .hover(|style| style.bg(rgb(0xf6f9f9)))
                .on_click(cx.listener(SettingsWindow::toggle_entitlement_type_picker))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_ellipsis()
                        .text_xs()
                        .text_color(rgb(0x405057))
                        .child(selected.clone()),
                )
                .child(chevron(open)),
        )
        .when(open, |this| {
            this.child(
                div().absolute().top_0().left_0().w(px(0.)).h(px(0.)).child(
                    deferred(
                        anchored()
                            .snap_to_window_with_margin(px(8.))
                            .position_mode(AnchoredPositionMode::Local)
                            .position(point(px(0.), px(34.)))
                            .child(
                                div()
                                    .w(px(160.))
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0xd8e0df))
                                    .bg(rgb(0xf8fbfa))
                                    .occlude()
                                    .child(dropdown_list(
                                        EntitlementValue::EDITABLE_TYPE_LABELS
                                            .into_iter()
                                            .enumerate()
                                            .map(|(index, label)| {
                                                entitlement_type_option(
                                                    index,
                                                    label,
                                                    label == selected.as_ref(),
                                                    cx,
                                                )
                                            }),
                                    )),
                            ),
                    )
                    .with_priority(10),
                ),
            )
        })
}

fn entitlement_type_option(
    index: usize,
    label: &'static str,
    selected: bool,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    div()
        .id(("entitlement-type-option", index))
        .h_7()
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
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xf6f9f9)))
        .on_click(cx.listener(move |settings, event, window, cx| {
            settings.select_entitlement_type(label, event, window, cx)
        }))
        .child(label)
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
