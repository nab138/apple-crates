use super::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    input::{Input, OtpInput},
    Disableable as _, Sizable as _,
};

pub(super) fn render(
    focus_handle: &FocusHandle,
    scroll_handle: &ScrollHandle,
    login: &DeveloperLoginState,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let content = div().flex().flex_col().gap_4().child(match login.step {
        DeveloperLoginStep::Credentials => credentials_form(login, cx),
        DeveloperLoginStep::SecondaryAction => secondary_action_form(login, cx),
    });

    settings_window_shell()
        .track_focus(focus_handle)
        .capture_key_down(cx.listener(SettingsWindow::handle_developer_login_key))
        .gap_4()
        .child(settings_window_header(
            "Add Apple Account",
            "Sign in before choosing developer teams, App IDs, and signing resources.",
        ))
        .child(scroll_panel(
            "developer-login-scroll",
            scroll_handle,
            content,
        ))
}

fn credentials_form(login: &DeveloperLoginState, cx: &mut Context<SettingsWindow>) -> gpui::Div {
    developer_login_card()
        .child(login_error(login.error.as_ref()))
        .child(login_input("Email", &login.email, login.busy))
        .child(password_input("Password", &login.password, login.busy))
        .child(remember_account_checkbox(
            login.remember_account,
            login.busy,
            cx,
        ))
        .child(primary_login_button(
            if login.busy {
                "Signing In..."
            } else {
                "Continue"
            },
            login.busy,
            cx.listener(SettingsWindow::submit_developer_login),
        ))
}

fn secondary_action_form(
    login: &DeveloperLoginState,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    developer_login_card()
        .child(
            div()
                .p_3()
                .rounded_md()
                .bg(rgb(0xf6f9f9))
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x24333a))
                        .child("Additional Authentication"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6a7a81))
                        .child(login.secondary_action_detail.clone()),
                ),
        )
        .child(login_error(login.error.as_ref()))
        .child(otp_input("Code, if requested", &login.code, login.busy))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(secondary_login_button(
                    "Back",
                    login.busy,
                    cx.listener(SettingsWindow::back_to_developer_login),
                ))
                .child(primary_login_button(
                    "Continue",
                    login.busy,
                    cx.listener(SettingsWindow::submit_developer_secondary_action),
                )),
        )
}

fn developer_login_card() -> gpui::Div {
    div()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xd8e0df))
        .bg(rgb(0xffffff))
        .flex()
        .flex_col()
        .gap_3()
}

fn login_input(label: &'static str, input: &Entity<InputState>, disabled: bool) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(settings_label(label))
        .child(Input::new(input).small().disabled(disabled))
}

fn password_input(label: &'static str, input: &Entity<InputState>, disabled: bool) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(settings_label(label))
        .child(Input::new(input).small().mask_toggle().disabled(disabled))
}

fn otp_input(label: &'static str, input: &Entity<OtpState>, disabled: bool) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(settings_label(label))
        .child(OtpInput::new(input).small().groups(2).disabled(disabled))
}

fn login_error(error: Option<&SharedString>) -> gpui::Div {
    div().when_some(error, |this, error| {
        this.p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe8c6c2))
            .bg(rgb(0xfff7f6))
            .text_sm()
            .text_color(rgb(0x8b302b))
            .child(error.clone())
    })
}

fn remember_account_checkbox(
    checked: bool,
    disabled: bool,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    Checkbox::new("remember-developer-account")
        .p_2()
        .rounded_md()
        .bg(rgb(0xf6f9f9))
        .hover(|style| style.bg(rgb(0xebf1f0)))
        .items_center()
        .small()
        .checked(checked)
        .disabled(disabled)
        .on_click(cx.listener(|this, checked: &bool, _, cx| {
            this.developer_login.remember_account = *checked;
            cx.notify();
        }))
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
                        .child("Save my account"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6a7a81))
                        .child("Store reusable account tokens in the system keychain."),
                ),
        )
}

fn primary_login_button(
    label: &'static str,
    disabled: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new("developer-login-primary")
        .primary()
        .small()
        .label(label)
        .disabled(disabled)
        .on_click(listener)
}

fn secondary_login_button(
    label: &'static str,
    disabled: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new("developer-login-secondary")
        .outline()
        .small()
        .label(label)
        .disabled(disabled)
        .on_click(listener)
}
