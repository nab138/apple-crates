mod assets;
mod constants;
mod data;
mod main_view;
mod models;
mod settings;
mod widgets;

use assets::Assets;
use constants::{WINDOW_HEIGHT, WINDOW_WIDTH};
use gpui::{px, size, App, AppContext, Bounds, QuitMode, WindowBounds, WindowOptions};
use main_view::SideloaderView;

fn main() {
    env_logger::init();

    gpui_platform::application()
        .with_assets(Assets)
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(|cx: &mut App| {
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
