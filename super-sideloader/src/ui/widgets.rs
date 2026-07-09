use crate::app::models::{AppOption, DeviceOption};
use crate::constants::*;
use crate::ui::theme::{action_surface_rgb, content_rgb, fixed_rgb, rgb, tokens};
use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, svg, AnchoredPositionMode, App,
    ElementId, FontWeight, IntoElement, KeyDownEvent, ParentElement, PathBuilder, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::Sizable as _;
use std::rc::Rc;

type PopoverOpenCallback = dyn Fn(&bool, &mut Window, &mut App);

#[derive(Clone, Copy)]
pub(crate) struct FloatingPopoverLayout {
    pub(crate) width: f32,
    pub(crate) max_height: f32,
    pub(crate) offset_y: f32,
}

pub(crate) fn progress_circle(progress: f32, turn: f32) -> impl IntoElement {
    let progress = progress.clamp(0., 1.);
    let turn = turn.fract();

    canvas(
        move |_, _, _| {},
        move |bounds, _, window, _| {
            let side = bounds.size.width.as_f32().min(bounds.size.height.as_f32());
            let radius = (side / 2. - 2.).max(1.);
            let center = point(
                bounds.origin.x + bounds.size.width / 2.,
                bounds.origin.y + bounds.size.height / 2.,
            );
            let radii = point(px(radius), px(radius));
            let top = point(center.x, center.y - px(radius));
            let bottom = point(center.x, center.y + px(radius));

            let mut track = PathBuilder::stroke(px(2.));
            track.move_to(top);
            track.arc_to(radii, px(0.), false, true, bottom);
            track.arc_to(radii, px(0.), false, true, top);
            if let Ok(path) = track.build() {
                window.paint_path(path, rgb(0x5b777d));
            }

            if progress <= 0. {
                return;
            }

            let start_angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * turn;
            let start = point(
                center.x + px(start_angle.cos() * radius),
                center.y + px(start_angle.sin() * radius),
            );
            let end_angle = start_angle + std::f32::consts::TAU * progress;
            let end = point(
                center.x + px(end_angle.cos() * radius),
                center.y + px(end_angle.sin() * radius),
            );

            let mut arc = PathBuilder::stroke(px(2.));
            arc.move_to(start);
            if progress >= 0.999 {
                let mid_angle = start_angle + std::f32::consts::PI;
                let mid = point(
                    center.x + px(mid_angle.cos() * radius),
                    center.y + px(mid_angle.sin() * radius),
                );
                arc.arc_to(radii, px(0.), false, true, mid);
                arc.arc_to(radii, px(0.), false, true, start);
            } else {
                arc.arc_to(radii, px(0.), progress > 0.5, true, end);
            }

            if let Ok(path) = arc.build() {
                window.paint_path(path, fixed_rgb(0xffffff));
            }
        },
    )
    .w_4()
    .h_4()
}
pub(crate) fn select_button(
    id: impl Into<ElementId>,
    overline: impl Into<SharedString>,
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    open: bool,
) -> Button {
    let overline = overline.into();
    let title = title.into();
    let detail = detail.into();
    let colors = tokens();

    surface_button(id)
        .h(px(COMBO_ITEM_HEIGHT))
        .w_full()
        .cursor_pointer()
        .child(
            div()
                .min_w_0()
                .size_full()
                .px_3()
                .rounded_md()
                .border_1()
                .border_color(if open {
                    colors.border_focus
                } else {
                    colors.border
                })
                .bg(colors.surface)
                .hover(move |style| style.bg(colors.surface_hover))
                .flex()
                .items_center()
                .gap_3()
                .child(select_item_content(overline, title, detail))
                .child(chevron(open)),
        )
}

pub(crate) fn select_option_button(
    id: impl Into<ElementId>,
    selected: bool,
    content: impl IntoElement,
) -> Button {
    let colors = tokens();
    surface_button(id)
        .h(px(COMBO_ITEM_HEIGHT))
        .w_full()
        .cursor_pointer()
        .child(
            div()
                .min_w_0()
                .size_full()
                .px_3()
                .rounded_md()
                .border_1()
                .border_color(if selected {
                    colors.border_focus
                } else {
                    colors.border
                })
                .bg(if selected {
                    colors.accent_surface
                } else {
                    colors.surface
                })
                .hover(move |style| style.bg(colors.surface_hover))
                .flex()
                .items_center()
                .child(content),
        )
}

pub(crate) fn select_with_popover(
    scroll_id: &'static str,
    trigger: Button,
    open: bool,
    on_open_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    popover: impl IntoElement,
) -> impl IntoElement {
    floating_select_popover(
        scroll_id,
        trigger,
        open,
        on_open_change,
        FloatingPopoverLayout {
            width: COMBO_POPOVER_WIDTH,
            max_height: COMBO_POPOVER_MAX_HEIGHT,
            offset_y: COMBO_POPOVER_OFFSET_Y,
        },
        popover,
    )
}

pub(crate) fn floating_select_popover(
    scroll_id: &'static str,
    trigger: Button,
    open: bool,
    on_open_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    layout: FloatingPopoverLayout,
    popover: impl IntoElement,
) -> gpui::Div {
    let on_open_change: Rc<PopoverOpenCallback> = Rc::new(on_open_change);
    let click_callback = on_open_change.clone();
    let key_callback = on_open_change.clone();
    let dismiss_callback = on_open_change.clone();
    let popover_key_callback = on_open_change.clone();
    let colors = tokens();

    let trigger = trigger
        .on_click(move |_, window, cx| {
            click_callback(&!open, window, cx);
        })
        .capture_key_down(move |event: &KeyDownEvent, window, cx| {
            match event.keystroke.key.as_str() {
                "enter" | "space" | "down" => {
                    cx.stop_propagation();
                    window.prevent_default();
                    key_callback(&true, window, cx);
                }
                "escape" if open => {
                    cx.stop_propagation();
                    window.prevent_default();
                    key_callback(&false, window, cx);
                }
                _ => {}
            }
        });

    div().relative().child(trigger).when(open, |this| {
        this.child(
            div().absolute().top_0().left_0().w(px(0.)).h(px(0.)).child(
                deferred(
                    anchored()
                        .position_mode(AnchoredPositionMode::Local)
                        .snap_to_window_with_margin(px(8.))
                        .position(point(px(0.), px(layout.offset_y)))
                        .child(
                            div()
                                .id(scroll_id)
                                .w(px(layout.width))
                                .max_h(px(layout.max_height))
                                .overflow_y_scroll()
                                .scrollbar_width(px(8.))
                                .rounded_md()
                                .border_1()
                                .border_color(colors.border)
                                .bg(colors.popover)
                                .occlude()
                                .tab_group()
                                .on_mouse_down_out(move |_, window, cx| {
                                    dismiss_callback(&false, window, cx);
                                })
                                .capture_key_down(move |event: &KeyDownEvent, window, cx| {
                                    if event.keystroke.key.as_str() == "escape" {
                                        cx.stop_propagation();
                                        window.prevent_default();
                                        popover_key_callback(&false, window, cx);
                                    }
                                })
                                .child(popover),
                        ),
                )
                .with_priority(1),
            ),
        )
    })
}

pub(crate) fn floating_menu_under(
    trigger: impl IntoElement,
    open: bool,
    on_open_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    width: f32,
    offset_y: f32,
    popover: impl IntoElement,
) -> gpui::Div {
    let on_open_change: Rc<PopoverOpenCallback> = Rc::new(on_open_change);
    let dismiss_callback = on_open_change.clone();
    let popover_key_callback = on_open_change.clone();
    let colors = tokens();

    div().relative().child(trigger).when(open, |this| {
        this.child(
            div().absolute().top_0().left_0().w(px(0.)).h(px(0.)).child(
                deferred(
                    anchored()
                        .position_mode(AnchoredPositionMode::Local)
                        .snap_to_window_with_margin(px(8.))
                        .position(point(px(0.), px(offset_y)))
                        .child(
                            div()
                                .w(px(width))
                                .rounded_md()
                                .border_1()
                                .border_color(colors.border)
                                .bg(colors.popover)
                                .overflow_hidden()
                                .occlude()
                                .tab_group()
                                .on_mouse_down_out(move |_, window, cx| {
                                    dismiss_callback(&false, window, cx);
                                })
                                .capture_key_down(move |event: &KeyDownEvent, window, cx| {
                                    if event.keystroke.key.as_str() == "escape" {
                                        cx.stop_propagation();
                                        window.prevent_default();
                                        popover_key_callback(&false, window, cx);
                                    }
                                })
                                .child(popover),
                        ),
                )
                .with_priority(1),
            ),
        )
    })
}

pub(crate) fn select_item_content(
    overline: impl Into<SharedString>,
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
) -> gpui::Div {
    let colors = tokens();
    div()
        .min_w_0()
        .h_full()
        .py_2()
        .flex()
        .flex_1()
        .flex_col()
        .justify_between()
        .child(
            div()
                .text_xs()
                .text_color(colors.text_secondary)
                .text_ellipsis()
                .child(overline.into()),
        )
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.text_primary)
                .text_ellipsis()
                .child(title.into()),
        )
        .child(
            div()
                .text_xs()
                .text_color(colors.text_muted)
                .text_ellipsis()
                .child(detail.into()),
        )
}

pub(crate) fn chevron(open: bool) -> impl IntoElement {
    let colors = tokens();
    div()
        .flex_none()
        .w_6()
        .h_6()
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .bg(colors.neutral_surface)
        .text_color(colors.text_secondary)
        .child(lucide_icon(if open {
            "icons/chevron-up.svg"
        } else {
            "icons/chevron-down.svg"
        }))
}

pub(crate) fn lucide_icon(path: &'static str) -> impl IntoElement {
    lucide_icon_tinted(path, 0x53666d)
}

pub(crate) fn lucide_icon_tinted(path: &'static str, color: u32) -> impl IntoElement {
    svg()
        .path(path)
        .size_4()
        .flex_none()
        .text_color(content_rgb(color))
}

pub(crate) fn connector_arrow_icon(color: u32) -> impl IntoElement {
    svg()
        .path("icons/arrow-right.svg")
        .size_6()
        .flex_none()
        .text_color(rgb(color))
}

pub(crate) fn developer_account_title(role: &str) -> SharedString {
    let role = role.trim();
    if role.is_empty() {
        "Developer account".into()
    } else {
        format!("{role} Developer account").into()
    }
}

pub(crate) fn app_identity(app: &AppOption) -> SharedString {
    format!("{} - {} ({})", app.bundle_id(), app.version(), app.build()).into()
}

pub(crate) fn device_identity(device: &DeviceOption) -> SharedString {
    format!("{} - {}", device.model, device.os).into()
}

pub(crate) fn dropdown_list<I, E>(options: I) -> gpui::Div
where
    I: IntoIterator<Item = E>,
    E: IntoElement,
{
    div().flex().flex_col().gap_2().p_2().children(options)
}

pub(crate) fn icon_button_surface(
    icon: &'static str,
    bg: u32,
    hover_bg: u32,
    color: u32,
) -> gpui::Div {
    square_button_surface(bg, hover_bg, color, lucide_icon_tinted(icon, color))
}

pub(crate) fn square_button_surface(
    bg: u32,
    hover_bg: u32,
    color: u32,
    child: impl IntoElement,
) -> gpui::Div {
    div()
        .size_full()
        .rounded_md()
        .bg(rgb(bg))
        .text_color(content_rgb(color))
        .hover(|style| style.bg(rgb(hover_bg)))
        .flex()
        .items_center()
        .justify_center()
        .child(child)
}

pub(crate) fn surface_button(id: impl Into<ElementId>) -> Button {
    let colors = tokens();
    Button::new(id)
        .text()
        .xsmall()
        .text_color(colors.text_primary)
}

pub(crate) fn primary_action_button_surface(
    label: impl Into<SharedString>,
    leading: Option<impl IntoElement>,
) -> gpui::Div {
    action_button_surface(label, 0x173f45, 0x20545c, 0xffffff, leading)
}

pub(crate) fn action_button_surface(
    label: impl Into<SharedString>,
    bg: u32,
    hover_bg: u32,
    color: u32,
    leading: Option<impl IntoElement>,
) -> gpui::Div {
    let label = label.into();

    div()
        .size_full()
        .px_3()
        .rounded_md()
        .bg(action_surface_rgb(bg, color))
        .hover(move |style| style.bg(action_surface_rgb(hover_bg, color)))
        .text_color(content_rgb(color))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .children(leading)
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .child(label),
        )
}

pub(crate) fn select_action_row(
    id: impl Into<ElementId>,
    title: &'static str,
    detail: &'static str,
) -> Button {
    let colors = tokens();
    surface_button(id).w_full().cursor_pointer().child(
        div()
            .min_w_0()
            .w_full()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .hover(move |style| style.bg(colors.surface_hover))
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.accent)
                            .child(title),
                    )
                    .child(div().text_xs().text_color(colors.text_muted).child(detail)),
            )
            .child(
                div()
                    .flex_none()
                    .w_6()
                    .h_6()
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(colors.neutral_surface)
                    .text_color(colors.text_secondary)
                    .child(lucide_icon("icons/plus.svg")),
            ),
    )
}

pub(crate) fn properties_list(rows: Vec<(&'static str, String)>) -> impl IntoElement {
    let colors = tokens();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .px_3()
        .children(rows.into_iter().map(|(label, value)| {
            div()
                .min_w_0()
                .flex()
                .items_start()
                .gap_3()
                .child(
                    div()
                        .w(px(92.))
                        .flex_none()
                        .text_xs()
                        .text_color(colors.text_muted)
                        .child(label),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_sm()
                        .text_color(colors.text_secondary)
                        .text_ellipsis()
                        .child(value),
                )
        }))
}
