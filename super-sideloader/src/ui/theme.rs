use crate::app::preferences::ThemePreference;
use gpui::{rgb as gpui_rgb, App, Rgba, Window, WindowAppearance};
use gpui_component::{Theme as ComponentTheme, ThemeMode as ComponentThemeMode};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppThemeMode {
    Light,
    Dark,
}

thread_local! {
    static CURRENT_THEME: Cell<AppThemeMode> = const { Cell::new(AppThemeMode::Light) };
}

pub(crate) fn sync_window_theme(window: &mut Window, cx: &mut App, preference: ThemePreference) {
    let mode = mode_for_preference(preference, window.appearance());
    CURRENT_THEME.set(mode);

    let component_mode = match mode {
        AppThemeMode::Light => ComponentThemeMode::Light,
        AppThemeMode::Dark => ComponentThemeMode::Dark,
    };
    let needs_component_sync =
        !cx.has_global::<ComponentTheme>() || ComponentTheme::global(cx).mode != component_mode;
    if needs_component_sync {
        ComponentTheme::change(component_mode, Some(window), cx);
    }
}

pub(crate) fn rgb(hex: u32) -> Rgba {
    let hex = CURRENT_THEME.with(|theme| match theme.get() {
        AppThemeMode::Light => hex,
        AppThemeMode::Dark => dark_color(hex),
    });
    gpui_rgb(hex)
}

pub(crate) fn fixed_rgb(hex: u32) -> Rgba {
    gpui_rgb(hex)
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct ThemeTokens {
    pub(crate) surface: Rgba,
    pub(crate) surface_subtle: Rgba,
    pub(crate) surface_hover: Rgba,
    pub(crate) popover: Rgba,
    pub(crate) border: Rgba,
    pub(crate) border_focus: Rgba,
    pub(crate) text_primary: Rgba,
    pub(crate) text_secondary: Rgba,
    pub(crate) text_muted: Rgba,
    pub(crate) accent: Rgba,
    pub(crate) accent_surface: Rgba,
    pub(crate) neutral_surface: Rgba,
    pub(crate) danger: Rgba,
    pub(crate) danger_surface: Rgba,
    pub(crate) danger_surface_hover: Rgba,
    pub(crate) success: Rgba,
    pub(crate) success_surface: Rgba,
    pub(crate) warning: Rgba,
    pub(crate) warning_surface: Rgba,
}

pub(crate) fn tokens() -> ThemeTokens {
    ThemeTokens {
        surface: rgb(0xffffff),
        surface_subtle: rgb(0xf6f9f9),
        surface_hover: rgb(0xeef6f5),
        popover: rgb(0xf8fbfa),
        border: rgb(0xcfd8d6),
        border_focus: rgb(0x0f6f7a),
        text_primary: rgb(0x24333a),
        text_secondary: rgb(0x405057),
        text_muted: rgb(0x7b8a90),
        accent: rgb(0x0f6f7a),
        accent_surface: rgb(0xf0fbfa),
        neutral_surface: rgb(0xebf1f0),
        danger: rgb(0x8b302b),
        danger_surface: rgb(0xf4e4e2),
        danger_surface_hover: rgb(0xebcecb),
        success: rgb(0x1d6b45),
        success_surface: rgb(0xd3eadc),
        warning: rgb(0x7a5613),
        warning_surface: rgb(0xfffbf3),
    }
}

pub(crate) fn content_rgb(hex: u32) -> Rgba {
    if hex == 0xffffff {
        fixed_rgb(hex)
    } else {
        rgb(hex)
    }
}

pub(crate) fn action_surface_rgb(hex: u32, foreground_hex: u32) -> Rgba {
    if foreground_hex == 0xffffff {
        fixed_rgb(hex)
    } else {
        rgb(hex)
    }
}

fn mode_for_preference(preference: ThemePreference, appearance: WindowAppearance) -> AppThemeMode {
    match preference {
        ThemePreference::System => mode_for_appearance(appearance),
        ThemePreference::Light => AppThemeMode::Light,
        ThemePreference::Dark => AppThemeMode::Dark,
    }
}

fn mode_for_appearance(appearance: WindowAppearance) -> AppThemeMode {
    match appearance {
        WindowAppearance::Light | WindowAppearance::VibrantLight => AppThemeMode::Light,
        WindowAppearance::Dark | WindowAppearance::VibrantDark => AppThemeMode::Dark,
    }
}

fn dark_color(hex: u32) -> u32 {
    match hex {
        0xffffff => 0x182123,
        0xfbfcfb | 0xf8fbfa => 0x151d1f,
        0xf6f9f9 | 0xf4f6f4 => 0x101617,
        0xf0f2f2 | 0xebf1f0 => 0x243033,
        0xe8eeee => 0x2a383b,
        0xeef6f5 => 0x223236,
        0xf0fbfa => 0x14383c,
        0xd7efec => 0x164247,
        0xdfe8e6 => 0x2d3c40,
        0xd8e0df | 0xd7dfdc | 0xcfd8d6 => 0x344347,
        0xe0d3d1 => 0x463633,
        0xe8c6c2 | 0xe7c8c5 | 0xe5b8b2 => 0x5a312e,
        0xfff7f6 | 0xfff5f3 => 0x2f1716,
        0xf4e4e2 | 0xebcecb => 0x3b1f1d,
        0xfffbf3 => 0x2d2111,
        0xf6edda | 0xeadbb8 | 0xe6d7bc => 0x3a2b13,
        0xe5f3ec | 0xd3eadc | 0xc5e2d1 => 0x173828,

        0x263238 | 0x24333a | 0x33454c | 0x405057 | 0x173f45 => 0xe3eceb,
        0x53666d | 0x66767c | 0x6a7a81 | 0x718086 | 0x73858b | 0x7b8a90 | 0x87959a => 0x9fb0b4,

        0x0f6f7a => 0x32b6c4,
        0x14393f | 0x20545c | 0x20565d => 0x1f6a74,
        0x168291 => 0x38b7c4,
        0x1d6b45 => 0x4fc987,
        0x5b777d => 0x86a3aa,

        0x8b302b | 0x8f3b35 | 0x9a302b | 0x8a4a46 | 0x8b6b67 | 0x7d3430 => 0xf09a92,
        0x7a5613 | 0x7a6a50 | 0x9a6a14 => 0xe4c27a,
        0xf4e9e7 | 0xe2c0bd => 0x3b1f1d,

        _ => hex,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_tokens_change_between_light_and_dark_modes() {
        CURRENT_THEME.set(AppThemeMode::Light);
        let light = tokens();
        CURRENT_THEME.set(AppThemeMode::Dark);
        let dark = tokens();
        CURRENT_THEME.set(AppThemeMode::Light);

        assert_ne!(light.surface, dark.surface);
        assert_ne!(light.text_primary, dark.text_primary);
        assert_ne!(light.border, dark.border);
    }

    #[test]
    fn filled_action_surfaces_keep_fixed_backgrounds_in_dark_mode() {
        CURRENT_THEME.set(AppThemeMode::Dark);
        assert_eq!(action_surface_rgb(0x173f45, 0xffffff), fixed_rgb(0x173f45));
        assert_eq!(action_surface_rgb(0xebf1f0, 0x53666d), rgb(0xebf1f0));
        CURRENT_THEME.set(AppThemeMode::Light);
    }
}
