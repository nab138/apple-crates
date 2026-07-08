use super::*;

pub(super) fn render(
    focus_handle: &FocusHandle,
    scroll_handle: &ScrollHandle,
    login: &DeveloperLoginState,
    cx: &mut Context<SettingsWindow>,
) -> gpui::Div {
    let content = div().flex().flex_col().gap_4().child(match login.step {
        DeveloperLoginStep::Credentials => credentials_form(login, cx),
        DeveloperLoginStep::TwoFactor => two_factor_form(login, cx),
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
        .child(login_input("Email", &login.email))
        .child(login_input("Password", &login.password))
        .child(remember_account_checkbox(login.remember_account, cx))
        .child(primary_login_button(
            "Continue",
            cx.listener(SettingsWindow::submit_developer_login),
        ))
        .child(mock_accounts_note())
}

fn two_factor_form(login: &DeveloperLoginState, cx: &mut Context<SettingsWindow>) -> gpui::Div {
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
                        .child("Two-Factor Authentication"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6a7a81))
                        .child(login.two_factor_detail.clone()),
                ),
        )
        .child(login_error(login.error.as_ref()))
        .child(login_input("Verification code", &login.code))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(secondary_login_button(
                    "Back",
                    cx.listener(SettingsWindow::back_to_developer_login),
                ))
                .child(primary_login_button(
                    "Verify",
                    cx.listener(SettingsWindow::submit_developer_two_factor),
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

fn login_input(label: &'static str, input: &Entity<EditLine>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(settings_label(label))
        .child(
            div()
                .h_8()
                .px_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xcdd8d6))
                .bg(rgb(0xffffff))
                .text_sm()
                .text_color(rgb(0x24333a))
                .flex()
                .items_center()
                .overflow_hidden()
                .child(input.clone()),
        )
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

fn remember_account_checkbox(checked: bool, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    div()
        .id("remember-developer-account")
        .p_2()
        .rounded_md()
        .bg(rgb(0xf6f9f9))
        .flex()
        .items_center()
        .gap_2()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xebf1f0)))
        .on_click(cx.listener(SettingsWindow::toggle_remember_developer_account))
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
                    this.text_color(rgb(0xffffff))
                        .child(lucide_icon("icons/check.svg"))
                }),
        )
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
                        .child("Use the account ID for system keyring storage when login is real."),
                ),
        )
}

fn primary_login_button(
    label: &'static str,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id("developer-login-primary")
        .h_9()
        .rounded_md()
        .bg(rgb(0x173f45))
        .text_color(rgb(0xffffff))
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x24565d)))
        .on_click(listener)
        .child(label)
}

fn secondary_login_button(
    label: &'static str,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id("developer-login-secondary")
        .h_9()
        .px_4()
        .rounded_md()
        .bg(rgb(0xebf1f0))
        .text_color(rgb(0x53666d))
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0xdfe8e6)))
        .on_click(listener)
        .child(label)
}

fn mock_accounts_note() -> gpui::Div {
    div()
        .p_3()
        .rounded_md()
        .bg(rgb(0xf6f9f9))
        .text_xs()
        .text_color(rgb(0x6a7a81))
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0x53666d))
                .child("Mock accounts"),
        )
        .child("a@example.com requires 2FA code 123456.")
        .child("b@example.com signs in without 2FA.")
}
