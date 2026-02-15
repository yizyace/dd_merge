use gpui::*;
use gpui_component::Root;
use gpui_component_assets::Assets;

use dd_ui::app_view::{OpenRepository, Quit};

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(|cx: &mut App| {
        gpui_component::init(cx);
        dd_ui::theme::setup_dark_theme(cx);

        cx.on_action(|_action: &Quit, cx: &mut App| {
            cx.quit();
        });

        cx.set_menus(vec![
            Menu {
                name: "DD Merge".into(),
                items: vec![MenuItem::action("Quit DD Merge", Quit)],
            },
            Menu {
                name: "File".into(),
                items: vec![MenuItem::action("Open Repository...", OpenRepository)],
            },
        ]);

        cx.activate(true);

        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        let _window_handle = cx
            .open_window(
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
                    let app_view_weak = app_view.downgrade();

                    // Handle File > Open Repository menu action
                    cx.on_action(move |_action: &OpenRepository, cx: &mut App| {
                        if let Some(app_view) = app_view_weak.upgrade() {
                            app_view.update(cx, |view, cx| {
                                view.open_repository_dialog(cx);
                            });
                        }
                    });

                    cx.new(|cx| Root::new(app_view, window, cx))
                },
            )
            .expect("failed to open window");
    });
}
