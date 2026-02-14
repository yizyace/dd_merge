use gpui::*;
use gpui_component::Root;
use gpui_component_assets::Assets;

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(|cx: &mut App| {
        gpui_component::init(cx);
        dd_ui::theme::setup_dark_theme(cx);
        cx.activate(true);

        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(Size {
                    width: px(480.),
                    height: px(320.),
                }),
                titlebar: Some(TitlebarOptions {
                    title: Some("DD Merge".into()),
                    ..Default::default()
                }),
                kind: WindowKind::Normal,
                ..Default::default()
            },
            |window, cx| {
                let app_view = cx.new(|cx| dd_ui::AppView::new(window, cx));
                cx.new(|cx| Root::new(app_view, window, cx))
            },
        )
        .expect("failed to open window");
    });
}
