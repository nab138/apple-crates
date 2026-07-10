use crate::ui::theme::tokens;
use gpui::{
    div, hsla, opaque_grey, point, px, App, AppContext as _, BoxShadow, Context, EventEmitter,
    FocusHandle, Focusable, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, PromptButton, PromptLevel, PromptResponse, Render, StatefulInteractiveElement,
    Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};

const PROMPT_WIDTH: f32 = 560.;
const PROMPT_BODY_MAX_HEIGHT: f32 = 260.;

pub(crate) fn install(cx: &mut App) {
    cx.set_prompt_builder(|level, message, detail, actions, handle, window, cx| {
        let prompt = cx.new(|cx| AppPrompt {
            level,
            message: message.to_string(),
            detail: detail.map(ToString::to_string),
            actions: actions.to_vec(),
            focus_handle: cx.focus_handle(),
        });

        handle.with_view(prompt, window, cx)
    });
}

struct AppPrompt {
    level: PromptLevel,
    message: String,
    detail: Option<String>,
    actions: Vec<PromptButton>,
    focus_handle: FocusHandle,
}

impl AppPrompt {
    fn respond(&mut self, index: usize, cx: &mut Context<Self>) {
        cx.emit(PromptResponse(index));
        cx.stop_propagation();
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let response = match event.keystroke.key.as_str() {
            "enter" => self.actions.iter().position(|action| !action.is_cancel()),
            "escape" => self.actions.iter().position(PromptButton::is_cancel),
            _ => None,
        };

        if let Some(index) = response {
            window.prevent_default();
            self.respond(index, cx);
        }
    }
}

impl Render for AppPrompt {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens();
        let (level_background, level_foreground, level_mark) = match self.level {
            PromptLevel::Info => (colors.accent_surface, colors.accent, "i"),
            PromptLevel::Warning => (colors.warning_surface, colors.warning, "!"),
            PromptLevel::Critical => (colors.danger_surface, colors.danger, "!"),
        };

        let header = div()
            .w_full()
            .flex()
            .items_start()
            .gap_3()
            .child(
                div()
                    .flex_none()
                    .size_10()
                    .rounded_full()
                    .bg(level_background)
                    .text_color(level_foreground)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child(level_mark),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .pt_1()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_primary)
                    .whitespace_normal()
                    .child(self.message.clone()),
            );

        let body = self.detail.clone().map(|detail| {
            div()
                .id("app-prompt-detail")
                .w_full()
                .max_h(px(PROMPT_BODY_MAX_HEIGHT))
                .overflow_hidden()
                .overflow_y_scroll()
                .scrollbar_width(px(8.))
                .rounded_md()
                .border_1()
                .border_color(colors.border)
                .bg(colors.surface_subtle)
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .p_3()
                        .text_sm()
                        .text_color(colors.text_secondary)
                        .whitespace_normal()
                        .child(detail),
                )
        });

        let actions = self
            .actions
            .iter()
            .enumerate()
            .rev()
            .map(|(index, action)| {
                let button = Button::new(("app-prompt-action", index))
                    .label(action.label().clone())
                    .on_click(cx.listener(move |prompt, _, _, cx| {
                        prompt.respond(index, cx);
                    }));

                if action.is_cancel() {
                    button.outline()
                } else if index == 0 {
                    button.primary()
                } else {
                    button.secondary()
                }
            });

        let dialog = div()
            .id("app-prompt")
            .track_focus(&self.focus_handle)
            .capture_key_down(cx.listener(Self::handle_key_down))
            .w(px(PROMPT_WIDTH))
            .max_w_full()
            .rounded_xl()
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_color(colors.text_primary)
            .shadow(vec![
                BoxShadow {
                    color: hsla(0., 0., 0., 0.18),
                    offset: point(px(0.), px(18.)),
                    blur_radius: px(40.),
                    spread_radius: px(-12.),
                    inset: false,
                },
                BoxShadow {
                    color: hsla(0., 0., 0., 0.1),
                    offset: point(px(0.), px(6.)),
                    blur_radius: px(14.),
                    spread_radius: px(-6.),
                    inset: false,
                },
            ])
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .p_5()
                    .pb_4()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(header)
                    .children(body),
            )
            .child(
                div()
                    .w_full()
                    .px_5()
                    .py_3()
                    .border_t_1()
                    .border_color(colors.border)
                    .bg(colors.surface_subtle)
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .children(actions),
            );

        div()
            .size_full()
            .absolute()
            .top_0()
            .left_0()
            .p_6()
            .bg(opaque_grey(0.05, 0.62))
            .flex()
            .items_center()
            .justify_center()
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
            .child(dialog)
    }
}

impl EventEmitter<PromptResponse> for AppPrompt {}

impl Focusable for AppPrompt {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_keyboard_actions_use_confirm_and_cancel_semantics() {
        let actions = [
            PromptButton::new("Continue"),
            PromptButton::Cancel("Cancel".into()),
        ];

        assert_eq!(
            actions.iter().position(|action| !action.is_cancel()),
            Some(0)
        );
        assert_eq!(actions.iter().position(PromptButton::is_cancel), Some(1));
    }
}
