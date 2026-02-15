use gpui::*;
use gpui_component::Root;
use gpui_component_assets::Assets;

use dd_core::Session;
use dd_ui::app_view::{CloseTab, NextTab, OpenRepository, PreviousTab, Quit};

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(|cx: &mut App| {
        gpui_component::init(cx);
        dd_ui::theme::setup_dark_theme(cx);

        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-o", OpenRepository, None),
            KeyBinding::new("cmd-w", CloseTab, None),
            KeyBinding::new("cmd-}", NextTab, None),
            KeyBinding::new("cmd-{", PreviousTab, None),
        ]);

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
                    let app_view_for_menu = app_view.downgrade();
                    let app_view_for_close = app_view.downgrade();
                    let app_view_for_next = app_view.downgrade();
                    let app_view_for_prev = app_view.downgrade();
                    let app_view_for_quit = app_view.downgrade();

                    // Handle File > Open Repository menu action
                    cx.on_action(move |_action: &OpenRepository, cx: &mut App| {
                        if let Some(app_view) = app_view_for_menu.upgrade() {
                            app_view.update(cx, |view, cx| {
                                view.open_repository_dialog(cx);
                            });
                        }
                    });

                    cx.on_action(move |_action: &CloseTab, cx: &mut App| {
                        if let Some(app_view) = app_view_for_close.upgrade() {
                            app_view.update(cx, |view, cx| {
                                view.close_active_tab(cx);
                            });
                        }
                    });

                    cx.on_action(move |_action: &NextTab, cx: &mut App| {
                        if let Some(app_view) = app_view_for_next.upgrade() {
                            app_view.update(cx, |view, cx| {
                                view.next_tab(cx);
                            });
                        }
                    });

                    cx.on_action(move |_action: &PreviousTab, cx: &mut App| {
                        if let Some(app_view) = app_view_for_prev.upgrade() {
                            app_view.update(cx, |view, cx| {
                                view.previous_tab(cx);
                            });
                        }
                    });

                    // Save session state on quit
                    let _ = cx.on_app_quit(move |cx| {
                        if let Some(app_view) = app_view_for_quit.upgrade() {
                            let state = app_view.read(cx).state().clone();
                            let _ = Session::save(&state);
                        }
                        async {}
                    });

                    cx.new(|cx| Root::new(app_view, window, cx))
                },
            )
            .expect("failed to open window");
    });
}
