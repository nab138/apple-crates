mod adi_backend;
mod adi_services;
mod app_selection;
mod assets;
mod constants;
mod data;
mod developer_accounts;
mod device_discovery;
mod device_selection;
mod edit_line;
mod ipa;
mod main_view;
mod models;
mod paths;
mod preferences;
mod settings;
mod system_identity;
mod widgets;

use assets::Assets;
use constants::{WINDOW_HEIGHT, WINDOW_WIDTH};
use gpui::{px, size, App, AppContext, Bounds, QuitMode, WindowBounds, WindowOptions};
use log::LevelFilter;
use main_view::SideloaderView;

fn main() {
    init_logging();

    gpui_platform::application()
        .with_assets(Assets)
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(|cx: &mut App| {
            edit_line::init(cx);

            let window_size = size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT));
            let bounds = Bounds::centered(None, window_size, cx);

            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(window_size),
                    is_resizable: true,
                    ..Default::default()
                },
                |window, cx| {
                    window.set_window_title("Super Sideloader");
                    cx.new(|cx| SideloaderView::new(window, cx))
                },
            )
            .expect("failed to open Super Sideloader window");

            cx.activate(true);
        });
}

fn init_logging() {
    let mut builder = env_logger::Builder::from_default_env();
    builder
        .filter_module("reqwest", LevelFilter::Info)
        .filter_module("hyper", LevelFilter::Info)
        .filter_module("h2", LevelFilter::Info)
        .filter_module("rustls", LevelFilter::Info)
        .filter_module("elf_loader", LevelFilter::Info);
    builder.init();
}
