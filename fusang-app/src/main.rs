use anyhow::Result;
use editor_ui_gpui::EditorView;
use gpui::{AppContext, Application, WindowOptions};

fn main() -> Result<()> {
    let app = Application::new();
    app.run(|app| {
        let window = app
            .open_window(WindowOptions::default(), |_window, cx| {
                cx.new(|cx| {
                    let mut view = EditorView::new(cx);
                    view.initialize(cx);
                    view
                })
            })
            .expect("failed to open window");

        let view = window
            .update(app, |_, _, cx| cx.entity())
            .expect("failed to get editor view");

        app.observe_keystrokes(move |event, _, cx| {
            view.update(cx, |view, cx| view.handle_key_event(&event, cx));
        })
        .detach();

        app.activate(true);
    });
    Ok(())
}
