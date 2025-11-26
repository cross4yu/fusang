use crate::AIPanel;
use editor_core_project::BufferManager;
use editor_core_text::CursorMovement;
use editor_infra::config::Config;
use gpui::{
    div, px, rgb, AppContext, AsyncApp, Context, Entity, IntoElement, KeystrokeEvent,
    ParentElement, Render, Styled, WeakEntity, Window,
};
use std::path::PathBuf;
use std::sync::Arc;

pub struct EditorView {
    buffer_manager: BufferManager,
    config: Config,
    current_file_path: Option<PathBuf>,
    show_ai_panel: bool,
    ai_panel: Option<Entity<AIPanel>>,
    ai_engine: Arc<editor_ai::AIEngine>,
}

impl EditorView {
    pub fn new(_cx: &mut Context<'_, Self>) -> Self {
        let config = Config::default();
        let ai_engine = Arc::new(editor_ai::AIEngine::new(config.ai.clone()));

        Self {
            buffer_manager: BufferManager::new(),
            config,
            current_file_path: None,
            show_ai_panel: false,
            ai_panel: None,
            ai_engine,
        }
    }

    /// 打开文件
    pub fn open_file(&mut self, file_path: &std::path::Path, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();
        let path = file_path.to_path_buf();

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            // ❗ clone 放在 async 之前
            let mut app = cx.clone();
            let path_for_io = path.clone();

            async move {
                match buffer_manager.open_file(&path_for_io).await {
                    Ok(_) => {
                        let path_clone = path_for_io.clone();
                        let _ = this.update(&mut app, |view, cx| {
                            view.current_file_path = Some(path_clone);
                            cx.notify();
                        });
                    }
                    Err(e) => log::error!("Failed to open file {}: {}", path_for_io.display(), e),
                }

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 插入文本
    pub fn insert_text(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();
        let text = text.to_string();

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();

            async move {
                if let Some(buffer_handle) = buffer_manager.get_current_buffer().await {
                    let mut buffer = buffer_handle.lock().await;
                    buffer.insert_text_at_cursor(&text).await;
                    let _ = this.update(&mut app, |_, cx| cx.notify());
                }

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 删除文本
    pub fn delete_text(&mut self, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();

            async move {
                if let Some(buffer_handle) = buffer_manager.get_current_buffer().await {
                    let mut buffer = buffer_handle.lock().await;
                    buffer.delete_backward().await;
                    let _ = this.update(&mut app, |_, cx| cx.notify());
                }

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 移动光标（占位）
    pub fn move_cursor(&mut self, _movement: CursorMovement, cx: &mut Context<'_, Self>) {
        log::info!("Move cursor placeholder");
        cx.notify();
    }

    /// 保存当前文件
    pub fn save_current_file(&mut self, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();

            async move {
                match buffer_manager.save_current_file().await {
                    Ok(_) => {
                        let _ = this.update(&mut app, |_, cx| cx.notify());
                    }
                    Err(e) => log::error!("Failed to save file: {}", e),
                }

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 获取当前文件路径
    pub fn current_file_path(&self) -> Option<&PathBuf> {
        self.current_file_path.as_ref()
    }

    /// 获取当前文件名称
    pub fn current_file_name(&self) -> Option<String> {
        self.current_file_path
            .as_ref()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
    }

    /// 获取文件语言
    pub fn current_file_language(&self) -> String {
        self.current_file_path
            .as_ref()
            .and_then(|p| p.extension().map(|e| e.to_string_lossy().to_string()))
            .unwrap_or_else(|| "text".to_string())
    }

    /// 切换 AI 面板显示
    pub fn toggle_ai_panel(&mut self, cx: &mut Context<'_, Self>) {
        self.show_ai_panel = !self.show_ai_panel;

        if self.show_ai_panel && self.ai_panel.is_none() {
            let ai_engine = self.ai_engine.clone();
            self.ai_panel = Some(cx.new(|cx| AIPanel::new(cx, ai_engine)));
        }

        cx.notify();
    }

    /// 设置 AI 面板上下文
    pub fn set_ai_context(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(ai_panel) = &self.ai_panel {
            let buffer_manager = self.buffer_manager.clone();
            let file_path = self.current_file_path.clone();
            let language = self.current_file_language();
            let ai_panel = ai_panel.clone();

            cx.spawn(move |_this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
                let mut app = cx.clone();

                async move {
                    if let Some(buffer_handle) = buffer_manager.get_current_buffer().await {
                        let buffer = buffer_handle.lock().await;
                        if let Ok(context) =
                            AIPanel::build_context_from_buffer(&buffer, file_path, &language).await
                        {
                            let _ = ai_panel.update(&mut app, move |panel, _| {
                                panel.set_buffer_context(context);
                            });
                        }
                    }

                    anyhow::Ok(())
                }
            })
            .detach();
        }
    }

    /// 向 AI 发送消息
    pub fn send_ai_message(&mut self, message: String, cx: &mut Context<'_, Self>) {
        if let Some(ai_panel) = &self.ai_panel {
            let ai_panel = ai_panel.clone();

            cx.spawn(move |_this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
                let mut app = cx.clone();

                async move {
                    if let Ok(mut panel_state) = ai_panel.update(&mut app, |panel, _| panel.clone())
                    {
                        // 如果这里将来报 E0282，就按 AIPanel 定义补 turbofish：
                        // panel_state.send_message::<AIPanelMessage>(message).await
                        if let Err(e) = panel_state.send_message(message).await {
                            log::error!("Failed to send AI message: {}", e);
                        }

                        let _ = ai_panel.update(&mut app, |panel, _| {
                            *panel = panel_state;
                        });
                    }

                    anyhow::Ok(())
                }
            })
            .detach();
        }
    }

    /// 请求代码解释
    pub fn request_code_explanation(&mut self, cx: &mut Context<'_, Self>) {
        self.set_ai_context(cx);
        self.send_ai_message("请解释这段代码的功能和工作原理。".to_string(), cx);
    }

    /// 请求代码改进
    pub fn request_code_improvements(&mut self, cx: &mut Context<'_, Self>) {
        self.set_ai_context(cx);
        self.send_ai_message("请分析这段代码并提供改进建议。".to_string(), cx);
    }

    /// 复制选中文本
    pub fn copy_selection(&mut self, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();

        cx.spawn(
            move |_this: WeakEntity<EditorView>, _cx: &mut AsyncApp| async move {
                if let Some(buffer_handle) = buffer_manager.get_current_buffer().await {
                    let buffer = buffer_handle.lock().await;
                    let selections = buffer.get_selections();
                    if let Some(selection) = selections.first() {
                        if !selection.is_collapsed() {
                            log::info!("Copy selection: {:?}", selection);
                        }
                    }
                }
                anyhow::Ok(())
            },
        )
        .detach();
    }

    /// 粘贴文本
    pub fn paste_text(&mut self, cx: &mut Context<'_, Self>) {
        log::info!("Paste text placeholder");
        cx.notify();
    }

    /// 撤销操作
    pub fn undo(&mut self, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();

            async move {
                if let Some(buffer_handle) = buffer_manager.get_current_buffer().await {
                    let mut buffer = buffer_handle.lock().await;
                    if buffer.undo().await {
                        let _ = this.update(&mut app, |_, cx| cx.notify());
                    }
                }

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 重做操作
    pub fn redo(&mut self, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();

            async move {
                if let Some(buffer_handle) = buffer_manager.get_current_buffer().await {
                    let mut buffer = buffer_handle.lock().await;
                    if buffer.redo().await {
                        let _ = this.update(&mut app, |_, cx| cx.notify());
                    }
                }

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 查找文本（占位）
    pub fn find_text(&mut self, query: &str, cx: &mut Context<'_, Self>) {
        log::info!("Find text: {}", query);
        cx.notify();
    }

    /// 替换文本（占位）
    pub fn replace_text(&mut self, query: &str, replacement: &str, cx: &mut Context<'_, Self>) {
        log::info!("Replace '{}' with '{}'", query, replacement);
        cx.notify();
    }

    /// 格式化代码（占位）
    pub fn format_code(&mut self, cx: &mut Context<'_, Self>) {
        log::info!("Format code placeholder");
        cx.notify();
    }

    /// 切换注释（占位）
    pub fn toggle_comment(&mut self, cx: &mut Context<'_, Self>) {
        log::info!("Toggle comment placeholder");
        cx.notify();
    }

    /// 缩进代码
    pub fn indent_code(&mut self, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();

            async move {
                if let Some(buffer_handle) = buffer_manager.get_current_buffer().await {
                    let mut buffer = buffer_handle.lock().await;
                    buffer.insert_tab(4).await; // TODO: use config.tab_size
                    let _ = this.update(&mut app, |_, cx| cx.notify());
                }

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 取消缩进代码（占位）
    pub fn unindent_code(&mut self, cx: &mut Context<'_, Self>) {
        log::info!("Unindent code placeholder");
        cx.notify();
    }
}

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let file_name = self
            .current_file_name()
            .unwrap_or_else(|| "Untitled".to_string());

        let mut layout = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xcccccc))
            .font_family("Monaco, Menlo, 'Courier New', monospace")
            .text_size(px(self.config.editor.font_size));

        layout = layout.child(
            div()
                .flex()
                .items_center()
                .p_2()
                .border_b_1()
                .border_color(rgb(0x333333))
                .child(div().flex_1().child(file_name))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(div().p_1().rounded(px(4.0)).bg(rgb(0x3a3a3a)).child("Save"))
                        .child(div().p_1().rounded(px(4.0)).bg(rgb(0x3a3a3a)).child("AI")),
                ),
        );

        let mut content_area = div().flex().flex_1();

        let editor_area = div().flex_1().p_4().child("Editor content will be here");

        content_area = content_area.child(editor_area);

        if self.show_ai_panel {
            if let Some(ai_panel) = &self.ai_panel {
                content_area = content_area.child(
                    div()
                        .w_96()
                        .border_l_1()
                        .border_color(rgb(0x333333))
                        .child(ai_panel.clone()),
                );
            }
        }

        layout.child(content_area)
    }
}

impl EditorView {
    pub fn handle_key_event(&mut self, event: &KeystrokeEvent, cx: &mut Context<'_, Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = &event.keystroke.modifiers;
        let command = modifiers.platform;

        match key {
            "s" if command => self.save_current_file(cx),
            "z" if command => self.undo(cx),
            "y" if command => self.redo(cx),
            "f" if command => log::info!("Open find dialog"),
            "c" if command => self.copy_selection(cx),
            "v" if command => self.paste_text(cx),
            "/" if command => self.toggle_comment(cx),
            "]" if command => self.indent_code(cx),
            "[" if command => self.unindent_code(cx),
            " " if modifiers.control => self.toggle_ai_panel(cx),
            _ => {
                if !modifiers.modified() {
                    match key {
                        "Backspace" => self.delete_text(cx),
                        "Enter" => self.insert_text("\n", cx),
                        "Tab" => self.indent_code(cx),
                        _ if event.keystroke.key.len() == 1 => {
                            self.insert_text(&event.keystroke.key, cx);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
