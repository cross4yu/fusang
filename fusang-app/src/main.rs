use anyhow::Result;
use editor_ui_gpui::EditorView;
use gpui::{AppContext, Application, WindowOptions};

fn main() -> Result<()> {
    let app = Application::new();
    app.run(|app| {
        let _ = app.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|cx| EditorView::new(cx))
        });
    });
    Ok(())
}
