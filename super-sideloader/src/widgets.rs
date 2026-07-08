use crate::constants::*;
use crate::models::{AppOption, DeviceOption};
use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, rgb, svg, AnchoredPositionMode,
    FontWeight, IntoElement, ParentElement, PathBuilder, SharedString, StatefulInteractiveElement,
    Styled,
};

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
                window.paint_path(path, rgb(0xffffff));
            }
        },
    )
    .w_4()
    .h_4()
}
pub(crate) fn combo_button(
    overline: impl Into<SharedString>,
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    open: bool,
) -> gpui::Div {
    let overline = overline.into();
    let title = title.into();
    let detail = detail.into();

    div()
        .h(px(COMBO_ITEM_HEIGHT))
        .px_3()
        .rounded_md()
        .border_1()
        .border_color(if open { rgb(0x0f6f7a) } else { rgb(0xcfd8d6) })
        .bg(rgb(0xffffff))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xf6f9f9)))
        .flex()
        .items_center()
        .gap_3()
        .child(combo_item_content(overline, title, detail))
        .child(chevron(open))
}

pub(crate) fn combo_with_popover(
    scroll_id: &'static str,
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
                        .position(point(px(0.), px(COMBO_POPOVER_OFFSET_Y)))
                        .child(
                            div()
                                .w(px(COMBO_POPOVER_WIDTH))
                                .max_h(px(COMBO_POPOVER_MAX_HEIGHT))
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(0xd8e0df))
                                .bg(rgb(0xf8fbfa))
                                .id(scroll_id)
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

pub(crate) fn combo_item_content(
    overline: impl Into<SharedString>,
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
) -> gpui::Div {
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
                .text_color(rgb(0x66767c))
                .text_ellipsis()
                .child(overline.into()),
        )
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0x24333a))
                .text_ellipsis()
                .child(title.into()),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x87959a))
                .text_ellipsis()
                .child(detail.into()),
        )
}

pub(crate) fn chevron(open: bool) -> impl IntoElement {
    div()
        .flex_none()
        .w_6()
        .h_6()
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(0xebf1f0))
        .text_color(rgb(0x53666d))
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
    svg().path(path).size_4().flex_none().text_color(rgb(color))
}

pub(crate) fn connector_arrow_icon(color: u32) -> impl IntoElement {
    svg()
        .path("icons/arrow-right.svg")
        .size_6()
        .flex_none()
        .text_color(rgb(color))
}

pub(crate) fn developer_account_title(role: &SharedString) -> SharedString {
    format!("{role} Developer account").into()
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

pub(crate) fn menu_action_row(title: &'static str, detail: &'static str) -> gpui::Div {
    div()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xcfd8d6))
        .bg(rgb(0xffffff))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xf0fbfa)))
        .child(
            div()
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
                                .text_color(rgb(0x173f45))
                                .child(title),
                        )
                        .child(div().text_xs().text_color(rgb(0x6a7a81)).child(detail)),
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
                        .bg(rgb(0xebf1f0))
                        .text_color(rgb(0x53666d))
                        .child(lucide_icon("icons/plus.svg")),
                ),
        )
}

pub(crate) fn properties_list(rows: Vec<(&'static str, String)>) -> impl IntoElement {
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
        }))
}
