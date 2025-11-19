use editor_core_project::BufferManager;
use editor_core_text::{Buffer, Cursor, CursorMovement, Selection};
use editor_infra::config::Config;
use editor_ui_gpui::AIPanel;
use gpui::*;
use std::path::PathBuf;
use std::sync::Arc;

pub struct EditorView {
    buffer_manager: BufferManager,
    config: Config,
    current_file_path: Option<PathBuf>,
    show_ai_panel: bool,
    ai_panel: Option<View<AIPanel>>,
    ai_engine: Arc<editor_ai::AIEngine>,
}

impl EditorView {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
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
    pub fn open_file(&mut self, file_path: &std::path::Path, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        let file_path = file_path.to_path_buf();
        
        cx.spawn(|mut cx| async move {
            if let Err(e) = buffer_manager.open_file(&file_path).await {
                log::error!("Failed to open file {}: {}", file_path.display(), e);
            } else {
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach();
        
        self.current_file_path = Some(file_path);
    }

    /// 获取当前缓冲区
    pub async fn get_current_buffer(&self) -> Option<Buffer> {
        self.buffer_manager.get_current_buffer().await.ok()
    }

    /// 插入文本
    pub fn insert_text(&mut self, text: &str, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        let text = text.to_string();
        
        cx.spawn(|mut cx| async move {
            if let Some(mut buffer) = buffer_manager.get_current_buffer().await.ok() {
                buffer.insert_text_at_cursor(&text).await;
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 删除文本
    pub fn delete_text(&mut self, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        
        cx.spawn(|mut cx| async move {
            if let Some(mut buffer) = buffer_manager.get_current_buffer().await.ok() {
                buffer.delete_backward().await;
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 移动光标
    pub fn move_cursor(&mut self, movement: CursorMovement, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        
        cx.spawn(|mut cx| async move {
            if let Some(mut buffer) = buffer_manager.get_current_buffer().await.ok() {
                // 这里需要根据实际的 Buffer API 来实现光标移动
                // 暂时使用通知更新
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 保存当前文件
    pub fn save_current_file(&mut self, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        
        cx.spawn(|mut cx| async move {
            if let Err(e) = buffer_manager.save_current_file().await {
                log::error!("Failed to save file: {}", e);
            } else {
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
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
    pub fn toggle_ai_panel(&mut self, cx: &mut ViewContext<Self>) {
        self.show_ai_panel = !self.show_ai_panel;
        
        if self.show_ai_panel && self.ai_panel.is_none() {
            let ai_engine = self.ai_engine.clone();
            self.ai_panel = Some(cx.new_view(|cx| AIPanel::new(cx, ai_engine)));
        }
        
        cx.notify();
    }

    /// 设置 AI 面板上下文
    pub fn set_ai_context(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(ai_panel) = &self.ai_panel {
            let buffer_manager = self.buffer_manager.clone();
            let file_path = self.current_file_path.clone();
            let language = self.current_file_language();
            let ai_panel = ai_panel.clone();
            
            cx.spawn(|mut cx| async move {
                if let Some(buffer) = buffer_manager.get_current_buffer().await.ok() {
                    if let Ok(context) = AIPanel::build_context_from_buffer(
                        &buffer, 
                        file_path, 
                        &language
                    ).await {
                        cx.update(|cx| {
                            if let Some(mut panel) = ai_panel.update(cx, |panel, _| {
                                panel.set_buffer_context(context);
                                Some(panel.clone())
                            }) {
                                // 上下文设置成功
                            }
                        })?;
                    }
                }
                anyhow::Ok(())
            })
            .detach();
        }
    }

    /// 向 AI 发送消息
    pub fn send_ai_message(&mut self, message: String, cx: &mut ViewContext<Self>) {
        if let Some(ai_panel) = &self.ai_panel {
            let ai_panel = ai_panel.clone();
            
            cx.spawn(|mut cx| async move {
                if let Some(mut panel) = ai_panel.update(&mut cx, |panel, cx| {
                    Some((panel.clone(), cx.clone()))
                }) {
                    let (panel, cx_ref) = panel;
                    panel.update(&cx_ref, |panel, _| {
                        let panel_clone = panel.clone();
                        cx_ref.spawn(|_| async move {
                            if let Err(e) = panel_clone.send_message(message).await {
                                log::error!("Failed to send AI message: {}", e);
                            }
                        })
                        .detach();
                    });
                }
                anyhow::Ok(())
            })
            .detach();
        }
    }

    /// 请求代码解释
    pub fn request_code_explanation(&mut self, cx: &mut ViewContext<Self>) {
        self.set_ai_context(cx);
        self.send_ai_message("请解释这段代码的功能和工作原理。".to_string(), cx);
    }

    /// 请求代码改进
    pub fn request_code_improvements(&mut self, cx: &mut ViewContext<Self>) {
        self.set_ai_context(cx);
        self.send_ai_message("请分析这段代码并提供改进建议。".to_string(), cx);
    }

    /// 复制选中文本
    pub fn copy_selection(&mut self, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        
        cx.spawn(|mut cx| async move {
            if let Some(buffer) = buffer_manager.get_current_buffer().await.ok() {
                let selections = buffer.get_selections();
                if let Some(selection) = selections.first() {
                    if !selection.is_collapsed() {
                        // 这里需要实现选区文本复制逻辑
                        // 暂时记录日志
                        log::info!("Copy selection: {:?}", selection);
                    }
                }
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 粘贴文本
    pub fn paste_text(&mut self, cx: &mut ViewContext<Self>) {
        // 这里需要实现从剪贴板粘贴文本的逻辑
        // 暂时使用空实现
        log::info!("Paste text");
        cx.notify();
    }

    /// 撤销操作
    pub fn undo(&mut self, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        
        cx.spawn(|mut cx| async move {
            if let Some(mut buffer) = buffer_manager.get_current_buffer().await.ok() {
                // 这里需要实现撤销逻辑
                // 暂时使用通知
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 重做操作
    pub fn redo(&mut self, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        
        cx.spawn(|mut cx| async move {
            if let Some(mut buffer) = buffer_manager.get_current_buffer().await.ok() {
                // 这里需要实现重做逻辑
                // 暂时使用通知
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 查找文本
    pub fn find_text(&mut self, query: &str, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        let query = query.to_string();
        
        cx.spawn(|mut cx| async move {
            if let Some(buffer) = buffer_manager.get_current_buffer().await.ok() {
                // 这里需要实现文本查找逻辑
                log::info!("Find text: {}", query);
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 替换文本
    pub fn replace_text(&mut self, query: &str, replacement: &str, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        let query = query.to_string();
        let replacement = replacement.to_string();
        
        cx.spawn(|mut cx| async move {
            if let Some(mut buffer) = buffer_manager.get_current_buffer().await.ok() {
                // 这里需要实现文本替换逻辑
                log::info!("Replace '{}' with '{}'", query, replacement);
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 格式化代码
    pub fn format_code(&mut self, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        
        cx.spawn(|mut cx| async move {
            if let Some(mut buffer) = buffer_manager.get_current_buffer().await.ok() {
                // 这里需要实现代码格式化逻辑
                // 可以使用 LSP 或内置格式化器
                log::info!("Format code");
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 切换注释
    pub fn toggle_comment(&mut self, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        
        cx.spawn(|mut cx| async move {
            if let Some(mut buffer) = buffer_manager.get_current_buffer().await.ok() {
                // 这里需要实现注释切换逻辑
                log::info!("Toggle comment");
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 缩进代码
    pub fn indent_code(&mut self, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        
        cx.spawn(|mut cx| async move {
            if let Some(mut buffer) = buffer_manager.get_current_buffer().await.ok() {
                buffer.insert_tab(4).await; // 使用 4 空格缩进
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach();
    }

    /// 取消缩进代码
    pub fn unindent_code(&mut self, cx: &mut ViewContext<Self>) {
        let buffer_manager = self.buffer_manager.clone();
        
        cx.spawn(|mut cx| async move {
            if let Some(mut buffer) = buffer_manager.get_current_buffer().await.ok() {
                // 这里需要实现取消缩进逻辑
                log::info!("Unindent code");
                cx.update(|cx| {
                    cx.notify();
                })?;
            }
            anyhow::Ok(())
        })
        .detach();
    }
}

impl Render for EditorView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let file_name = self.current_file_name()
            .unwrap_or_else(|| "Untitled".to_string());
        
        let mut layout = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xcccccc))
            .font_family("Monaco, Menlo, 'Courier New', monospace")
            .font_size(self.config.editor.font_size);

        // 标题栏
        layout = layout.child(
            div()
                .flex()
                .items_center()
                .p_2()
                .border_b_1()
                .border_color(rgb(0x333333))
                .child(
                    div()
                        .flex_1()
                        .child(file_name)
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            button()
                                .on_click(cx.listener(|this, _, cx| this.save_current_file(cx)))
                                .child("Save")
                        )
                        .child(
                            button()
                                .on_click(cx.listener(|this, _, cx| this.toggle_ai_panel(cx)))
                                .child("AI")
                        )
                )
        );

        // 主内容区域
        let mut content_area = div()
            .flex()
            .flex_1();

        // 编辑器区域
        let editor_area = div()
            .flex_1()
            .p_4()
            .child("Editor content will be here")
            .on_click(cx.listener(|this, _, cx| {
                // 处理编辑器点击事件
                log::info!("Editor clicked");
            }));

        content_area = content_area.child(editor_area);

        // AI 面板
        if self.show_ai_panel {
            if let Some(ai_panel) = &self.ai_panel {
                content_area = content_area.child(
                    div()
                        .w_96()
                        .border_l_1()
                        .border_color(rgb(0x333333))
                        .child(ai_panel.clone())
                );
            }
        }

        layout.child(content_area)
    }
}

// 键盘事件处理
impl EditorView {
    pub fn handle_key_event(&mut self, event: &KeyEvent, cx: &mut ViewContext<Self>) {
        match event.key.as_str() {
            "s" if event.modifiers.command => {
                self.save_current_file(cx);
            }
            "z" if event.modifiers.command => {
                self.undo(cx);
            }
            "y" if event.modifiers.command => {
                self.redo(cx);
            }
            "f" if event.modifiers.command => {
                // 打开查找对话框
                log::info!("Open find dialog");
            }
            "c" if event.modifiers.command => {
                self.copy_selection(cx);
            }
            "v" if event.modifiers.command => {
                self.paste_text(cx);
            }
            "/" if event.modifiers.command => {
                self.toggle_comment(cx);
            }
            "]" if event.modifiers.command => {
                self.indent_code(cx);
            }
            "[" if event.modifiers.command => {
                self.unindent_code(cx);
            }
            " " if event.modifiers.control => {
                self.toggle_ai_panel(cx);
            }
            _ => {
                // 处理其他按键
                if event.modifiers.is_empty() {
                    match event.key.as_str() {
                        "Backspace" => {
                            self.delete_text(cx);
                        }
                        "Enter" => {
                            self.insert_text("\n", cx);
                        }
                        "Tab" => {
                            self.indent_code(cx);
                        }
                        _ if event.key.len() == 1 => {
                            self.insert_text(&event.key, cx);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
