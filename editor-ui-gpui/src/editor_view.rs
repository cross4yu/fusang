use crate::AIPanel;
use editor_core_project::BufferManager;
use editor_core_text::CursorMovement;
use editor_infra::config::Config;
use gpui::{
    div, px, rgb, AppContext, AsyncApp, Context, Entity, InteractiveElement, KeystrokeEvent,
    StatefulInteractiveElement, WeakEntity, Window, prelude::*,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use unicode_width::UnicodeWidthChar;

pub struct EditorView {
    buffer_manager: BufferManager,
    config: Config,
    current_file_path: Option<PathBuf>,
    open_files: Vec<PathBuf>,
    lines: Vec<String>,
    line_prefix_widths: Vec<Vec<f32>>,
    selection: Option<editor_core_text::Selection>,
    is_dirty: bool,
    rendered_text: String,
    status_message: String,
    show_ai_panel: bool,
    ai_panel: Option<Entity<AIPanel>>,
    ai_engine: Arc<editor_ai::AIEngine>,
    quick_open_active: bool,
    quick_open_input: String,
    ai_prompt_input: String,
    ai_input_focused: bool,
    dragging: bool,
    scroll_handle: gpui::ScrollHandle,
}

impl EditorView {
    pub fn new(_cx: &mut Context<'_, Self>) -> Self {
        let config = Config::default();
        let ai_engine = Arc::new(editor_ai::AIEngine::new(config.ai.clone()));

        Self {
            buffer_manager: BufferManager::new(),
            config,
            current_file_path: None,
            open_files: Vec::new(),
            lines: Vec::new(),
            line_prefix_widths: Vec::new(),
            selection: None,
            is_dirty: false,
            rendered_text: String::new(),
            status_message: "Bootstrapping workspace…".to_string(),
            show_ai_panel: false,
            ai_panel: None,
            ai_engine,
            quick_open_active: false,
            quick_open_input: String::new(),
            ai_prompt_input: String::new(),
            ai_input_focused: false,
            dragging: false,
            scroll_handle: gpui::ScrollHandle::new(),
        }
    }

    /// 启动时加载 README.md 或创建新的缓冲区，并写入欢迎文案
    pub fn initialize(&mut self, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();
        let welcome = Self::welcome_text();
        let tab_size = self.config.editor.tab_size;
        let repo_readme = std::env::current_dir()
            .ok()
            .map(|mut dir| {
                dir.push("README.md");
                dir
            })
            .filter(|path| path.exists());

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();
            async move {
                let target_path = if let Some(path) = repo_readme {
                    match buffer_manager.open_file(&path).await {
                        Ok(_) => path,
                        Err(_) => buffer_manager.create_new_buffer().await,
                    }
                } else {
                    buffer_manager.create_new_buffer().await
                };

                if let Some(buffer_handle) = buffer_manager.get_buffer(&target_path).await {
                    let mut buffer = buffer_handle.lock().await;
                    if buffer.get_text().await.is_empty() {
                        buffer.insert_text_at_cursor(&welcome).await;
                    }
                }

                let snapshot = if let Some(buffer_handle) = buffer_manager.get_buffer(&target_path).await {
                    let buffer = buffer_handle.lock().await;
                    buffer.get_text().await
                } else {
                    String::new()
                };

                let open_files = buffer_manager.get_open_files().await;
                let (lines, selection, is_dirty, widths) =
                    Self::snapshot_buffer(&buffer_manager, tab_size)
                        .await
                        .unwrap_or_default();

                let _ = this.update(&mut app, |view, cx| {
                    view.current_file_path = Some(target_path.clone());
                    view.rendered_text = snapshot;
                    view.open_files = open_files;
                    view.lines = lines;
                    view.line_prefix_widths = widths;
                    view.selection = selection;
                    view.is_dirty = is_dirty;
                    view.status_message = "Workspace ready".to_string();
                    cx.notify();
                });

                anyhow::Ok(())
            }
        })
        .detach();
    }

    async fn snapshot_buffer(
        buffer_manager: &BufferManager,
        tab_size: usize,
    ) -> Option<(
        Vec<String>,
        Option<editor_core_text::Selection>,
        bool,
        Vec<Vec<f32>>,
    )> {
        let handle = buffer_manager.get_current_buffer().await?;
        let buffer = handle.lock().await;
        let line_count = buffer.line_count().await;
        let mut lines = Vec::with_capacity(line_count);
        let mut widths = Vec::with_capacity(line_count);
        for i in 0..line_count {
            if let Some(line) = buffer.get_line(i).await {
                let mut prefix = Vec::with_capacity(line.chars().count());
                let mut acc = 0.0f32;
                for ch in line.chars() {
                    let w_units = if ch == '\t' {
                        tab_size as f32
                    } else {
                        UnicodeWidthChar::width(ch).unwrap_or(1) as f32
                    };
                    acc += w_units;
                    prefix.push(acc);
                }
                lines.push(line);
                widths.push(prefix);
            }
        }
        let selection = buffer.get_selections().first().cloned();
        let is_dirty = buffer.is_dirty();
        Some((lines, selection, is_dirty, widths))
    }

    fn welcome_text() -> String {
        [
            "// Fusang · Cursor-inspired shell",
            "// - 左侧切换打开的文件",
            "// - 编辑区直接键入，内容写入 BufferManager",
            "// - Cmd+S 保存，Cmd+Z/Y 撤销/重做，Ctrl+Space 切换 AI 面板",
            "",
        ]
        .join("\n")
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }

    fn refresh_buffer_view(&mut self, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();
        let tab_size = self.config.editor.tab_size;

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();

            async move {
                let text = if let Some(buffer_handle) = buffer_manager.get_current_buffer().await {
                    let buffer = buffer_handle.lock().await;
                    buffer.get_text().await
                } else {
                    String::new()
                };

                let open_files = buffer_manager.get_open_files().await;
                let current_path = buffer_manager.get_current_file_path().await;
                let (lines, selection, is_dirty, widths) =
                    Self::snapshot_buffer(&buffer_manager, tab_size)
                        .await
                        .unwrap_or_default();

                let _ = this.update(&mut app, |view, cx| {
                    view.rendered_text = text.clone();
                    view.open_files = open_files.clone();
                    view.current_file_path = current_path.clone();
                    view.line_prefix_widths = widths;
                    view.lines = lines;
                    view.selection = selection;
                    view.is_dirty = is_dirty;
                    cx.notify();
                });

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 打开文件
    pub fn open_file(&mut self, file_path: &Path, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();
        let path = file_path.to_path_buf();

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();
            let path_for_io = path.clone();

            async move {
                match buffer_manager.open_file(&path_for_io).await {
                    Ok(_) => {
                        let path_clone = path_for_io.clone();
                        let _ = this.update(&mut app, |view, cx| {
                            view.current_file_path = Some(path_clone);
                            view.set_status("文件已打开");
                            view.refresh_buffer_view(cx);
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
                    let _ = this.update(&mut app, |view, cx| {
                        view.set_status("已输入文本");
                        view.refresh_buffer_view(cx);
                        view.is_dirty = true;
                        cx.notify();
                    });
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
                    let _ = this.update(&mut app, |view, cx| {
                        view.set_status("删除字符");
                        view.refresh_buffer_view(cx);
                        view.is_dirty = true;
                        cx.notify();
                    });
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
                        let _ = this.update(&mut app, |view, cx| {
                            view.set_status("保存成功");
                            view.refresh_buffer_view(cx);
                            view.is_dirty = false;
                            cx.notify();
                        });
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
            self.set_ai_context(cx);
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
                        let _ = this.update(&mut app, |view, cx| {
                            view.set_status("撤销");
                            view.refresh_buffer_view(cx);
                            view.is_dirty = buffer.is_dirty();
                            cx.notify();
                        });
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
                        let _ = this.update(&mut app, |view, cx| {
                            view.set_status("重做");
                            view.refresh_buffer_view(cx);
                            view.is_dirty = buffer.is_dirty();
                            cx.notify();
                        });
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
                    let _ = this.update(&mut app, |view, cx| {
                        view.set_status("缩进");
                        view.refresh_buffer_view(cx);
                        view.is_dirty = true;
                        cx.notify();
                    });
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

    /// 创建一个新的临时缓冲区
    pub fn new_buffer(&mut self, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();
        let tab_size = self.config.editor.tab_size;

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();
            async move {
                let path = buffer_manager.create_new_buffer().await;
                let (lines, selection, is_dirty, widths) =
                    EditorView::snapshot_buffer(&buffer_manager, tab_size).await.unwrap_or_default();
                let text = if let Some(handle) = buffer_manager.get_buffer(&path).await {
                    let buffer = handle.lock().await;
                    buffer.get_text().await
                } else {
                    String::new()
                };

                let open_files = buffer_manager.get_open_files().await;

                let _ = this.update(&mut app, |view, cx| {
                    view.current_file_path = Some(path.clone());
                    view.open_files = open_files;
                    view.lines = lines;
                    view.line_prefix_widths = widths;
                    view.selection = selection;
                    view.is_dirty = is_dirty;
                    view.rendered_text = text;
                    view.status_message = "新建 untitled 缓冲区".to_string();
                    cx.notify();
                });

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 打开快速输入框并打开路径
    fn open_quick_input_path(&mut self, cx: &mut Context<'_, Self>) {
        let path_text = self.quick_open_input.trim().to_string();
        if path_text.is_empty() {
            self.quick_open_active = false;
            cx.notify();
            return;
        }

        let buffer_manager = self.buffer_manager.clone();
        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();
            let path_text = path_text.clone();

            async move {
                let mut target = PathBuf::from(&path_text);
                if target.is_relative() {
                    if let Ok(cwd) = std::env::current_dir() {
                        target = cwd.join(target);
                    }
                }

                let result = if target.exists() {
                    buffer_manager.open_file(&target).await
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "文件不存在",
                    ))
                };

                let _ = this.update(&mut app, |view, cx| {
                    if result.is_ok() {
                        view.current_file_path = Some(target.clone());
                        view.status_message = format!("打开 {}", target.display());
                        view.quick_open_active = false;
                        view.quick_open_input.clear();
                        view.refresh_buffer_view(cx);
                    } else {
                        view.status_message =
                            format!("无法打开 {}: {:?}", target.display(), result.err());
                    }
                    cx.notify();
                });

                anyhow::Ok(())
            }
        })
        .detach();
    }

    fn push_ai_prompt_char(&mut self, ch: &str, cx: &mut Context<'_, Self>) {
        self.ai_prompt_input.push_str(ch);
        cx.notify();
    }

    fn backspace_ai_prompt(&mut self, cx: &mut Context<'_, Self>) {
        self.ai_prompt_input.pop();
        cx.notify();
    }

    fn send_ai_prompt(&mut self, cx: &mut Context<'_, Self>) {
        if self.ai_prompt_input.trim().is_empty() {
            return;
        }
        let msg = self.ai_prompt_input.trim().to_string();
        self.set_ai_context(cx);
        self.send_ai_message(msg, cx);
        self.ai_prompt_input.clear();
        cx.notify();
    }

    /// 设置光标位置并可选扩展选区
    fn set_cursor_position(
        &mut self,
        line: usize,
        column: usize,
        extend: bool,
        cx: &mut Context<'_, Self>,
    ) {
        let buffer_manager = self.buffer_manager.clone();

        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();
            async move {
                if let Some(handle) = buffer_manager.get_current_buffer().await {
                    let mut buffer = handle.lock().await;
                    let current = buffer.get_selections().first().cloned();
                    let anchor = current
                        .as_ref()
                        .map(|s| s.anchor)
                        .unwrap_or(editor_core_text::Cursor::zero());
                    let new_cursor = editor_core_text::Cursor::new(line, column);
                    if extend {
                        buffer.set_selection(editor_core_text::Selection::new(anchor, new_cursor));
                    } else {
                        buffer.set_cursor(new_cursor);
                    }
                }

                let _ = this.update(&mut app, |view, cx| {
                    view.set_status("移动光标");
                    view.refresh_buffer_view(cx);
                    cx.notify();
                });

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 根据方向移动光标
    fn move_cursor_by(&mut self, movement: CursorMovement, extend: bool, cx: &mut Context<'_, Self>) {
        let buffer_manager = self.buffer_manager.clone();
        cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
            let mut app = cx.clone();
            async move {
                if let Some(handle) = buffer_manager.get_current_buffer().await {
                    let mut buffer = handle.lock().await;
                    let current = buffer
                        .get_selections()
                        .first()
                        .cloned()
                        .unwrap_or(editor_core_text::Selection::single(
                            editor_core_text::Cursor::zero(),
                        ));
                    let mut cursor = current.active;
                    let line_count = buffer.line_count().await;

                    match movement {
                        CursorMovement::Left => {
                            if cursor.column > 0 {
                                cursor.column -= 1;
                            } else if cursor.line > 0 {
                                cursor.line -= 1;
                                cursor.column = buffer.get_line_length(cursor.line).await.unwrap_or(0);
                            }
                        }
                        CursorMovement::Right => {
                            let len = buffer.get_line_length(cursor.line).await.unwrap_or(0);
                            if cursor.column < len {
                                cursor.column += 1;
                            } else if cursor.line + 1 < line_count {
                                cursor.line += 1;
                                cursor.column = 0;
                            } else {
                                cursor.column = len;
                            }
                        }
                        CursorMovement::Up => {
                            if cursor.line > 0 {
                                cursor.line -= 1;
                                let len = buffer.get_line_length(cursor.line).await.unwrap_or(0);
                                cursor.column = cursor.column.min(len);
                            }
                        }
                        CursorMovement::Down => {
                            let next_line = cursor.line + 1;
                            if next_line < line_count {
                                cursor.line = next_line;
                                let len = buffer.get_line_length(cursor.line).await.unwrap_or(0);
                                cursor.column = cursor.column.min(len);
                            }
                        }
                        CursorMovement::LineStart | CursorMovement::Home => {
                            cursor.column = 0;
                        }
                        CursorMovement::LineEnd | CursorMovement::End => {
                            cursor.column = buffer.get_line_length(cursor.line).await.unwrap_or(0);
                        }
                        _ => {}
                    }

                    if extend {
                        buffer.set_selection(editor_core_text::Selection::new(current.anchor, cursor));
                    } else {
                        buffer.set_cursor(cursor);
                    }
                }

                let _ = this.update(&mut app, |view, cx| {
                    view.set_status("移动光标");
                    view.refresh_buffer_view(cx);
                    cx.notify();
                });

                anyhow::Ok(())
            }
        })
        .detach();
    }

    /// 将点击位置转换为列号，基于大致字符宽度
    fn hit_test_column(&self, line_idx: usize, mouse_x: gpui::Pixels) -> usize {
        let font_px = self.config.editor.font_size as f32;
        let char_w = (font_px.max(8.0)) * 0.6;
        let pos_x: f32 = mouse_x.into();
        let scroll_x: f32 = self.scroll_handle.offset().x.into();
        let digits = ((self.lines.len().max(1) as f32).log10().floor() as usize) + 1;
        let gutter = char_w * digits as f32 + 16.0; // line numbers + padding
        let base_x = gutter + 16.0; // gap before code
        if pos_x + scroll_x <= base_x {
            return 0;
        }

        let Some(line) = self.lines.get(line_idx) else {
            return 0;
        };

        let target_units = (pos_x + scroll_x - base_x) / char_w;
        let mut acc = 0.0f32;
        for (idx, ch) in line.chars().enumerate() {
            let w_units = if ch == '\t' {
                self.config.editor.tab_size as f32
            } else {
                UnicodeWidthChar::width(ch).unwrap_or(1) as f32
            };
            if acc + w_units * 0.5 >= target_units {
                return idx;
            }
            acc += w_units;
        }

        line.chars().count()
    }

    /// 拖拽时靠近上下边缘自动滚动
    fn autoscroll_on_drag(&mut self, mouse_y: gpui::Pixels) {
        let view_bounds = self.scroll_handle.bounds();
        let pos_y: f32 = mouse_y.into();
        let top: f32 = view_bounds.top().into();
        let bottom: f32 = view_bounds.bottom().into();
        let threshold = 32.0;
        if pos_y < top + threshold {
            let current = self.scroll_handle.top_item();
            let target = current.saturating_sub(1);
            self.scroll_handle.scroll_to_top_of_item(target);
        } else if pos_y > bottom - threshold {
            let target = self.scroll_handle.bottom_item() + 1;
            self.scroll_handle.scroll_to_item(target);
        }
    }
}

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let file_name = self
            .current_file_name()
            .unwrap_or_else(|| "Untitled".to_string());
        let language = self.current_file_language();
        let ai_panel_open = self.show_ai_panel;
        let cursor = self.selection.map(|sel| sel.active);

        let save_listener = cx.listener(|view: &mut EditorView, _, _, cx| {
            view.save_current_file(cx)
        });
        let toggle_ai_listener = cx.listener(|view: &mut EditorView, _, _, cx| {
            view.toggle_ai_panel(cx)
        });
        let new_file_listener = cx.listener(|view: &mut EditorView, _, _, cx| {
            view.new_buffer(cx)
        });
        let quick_open_listener = cx.listener(|view: &mut EditorView, _, _, cx| {
            view.quick_open_active = true;
            view.quick_open_input.clear();
            view.status_message = "输入路径后回车打开，Esc 取消".to_string();
            cx.notify();
        });

        let mut sidebar = div()
            .w(px(200.0))
            .bg(rgb(0x161616))
            .border_r_1()
            .border_color(rgb(0x2a2a2a))
            .flex()
            .flex_col();

        sidebar = sidebar.child(
            div()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(rgb(0x2a2a2a))
                .text_color(rgb(0x9ad1ff))
                .text_sm()
                .child("Workspace"),
        );

        for (idx, path) in self.open_files.iter().enumerate() {
            let is_active = self
                .current_file_path
                .as_ref()
                .map(|p| p == path)
                .unwrap_or(false);

            let display = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());

            let path_clone = path.clone();
            let click_handler = cx.listener(move |view: &mut EditorView, _, _, cx| {
                let buffer_manager = view.buffer_manager.clone();
                let path = path_clone.clone();
                cx.spawn(move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
                    let mut app = cx.clone();
                    async move {
                        if buffer_manager.get_buffer(&path).await.is_some() {
                            let _ = buffer_manager.set_current_buffer(&path).await;
                        } else if path.exists() {
                            let _ = buffer_manager.open_file(&path).await;
                        }

                        let _ = this.update(&mut app, |view, cx| {
                            view.current_file_path = Some(path.clone());
                            view.set_status("切换文件");
                            view.refresh_buffer_view(cx);
                            cx.notify();
                        });

                        anyhow::Ok(())
                    }
                })
                .detach();
            });

            sidebar = sidebar.child(
                div()
                    .id(("sidebar", idx as u64))
                    .px_3()
                    .py_2()
                    .text_sm()
                    .rounded(px(6.0))
                    .bg(if is_active { rgb(0x1f1f1f) } else { rgb(0x161616) })
                    .text_color(if is_active { rgb(0xffffff) } else { rgb(0xbbbbbb) })
                    .cursor_pointer()
                    .child(display)
                    .on_click(click_handler),
            );
        }

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
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(rgb(0x2a2a2a))
                .bg(rgb(0x121212))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(div().text_color(rgb(0x8ef1a2)).child("Fusang"))
                        .child(
                            div()
                                .text_color(rgb(0x888888))
                                .text_sm()
                                .child(format!("{} • {}", language, file_name)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(
                            div()
                                .id("new-button")
                                .px_3()
                                .py_1()
                                .rounded(px(6.0))
                                .bg(rgb(0x3a3a3a))
                                .cursor_pointer()
                                .child("New")
                                .on_click(new_file_listener),
                        )
                        .child(
                            div()
                                .id("open-button")
                                .px_3()
                                .py_1()
                                .rounded(px(6.0))
                                .bg(rgb(0x3a3a3a))
                                .cursor_pointer()
                                .child("Open…")
                                .on_click(quick_open_listener),
                        )
                        .child(
                            div()
                                .id("save-button")
                                .px_3()
                                .py_1()
                                .rounded(px(6.0))
                                .bg(rgb(0x2e7d32))
                                .active(|btn| btn.opacity(0.85))
                                .cursor_pointer()
                                .child("Save")
                                .on_click(save_listener),
                        )
                        .child(
                            div()
                                .id("ai-toggle")
                                .px_3()
                                .py_1()
                                .rounded(px(6.0))
                                .bg(if ai_panel_open { rgb(0x1a4d8f) } else { rgb(0x3a3a3a) })
                                .active(|btn| btn.opacity(0.85))
                                .cursor_pointer()
                                .child(if ai_panel_open { "Hide AI" } else { "AI Copilot" })
                                .on_click(toggle_ai_listener),
                        ),
                ),
        );

        let mut content_area = div().flex().flex_1();

        content_area = content_area.child(sidebar);

        let editor_area = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0x0f0f0f))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_sm()
                    .text_color(rgb(0xaaaaaa))
                    .child(format!("{} ({})", file_name, language))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child("Cmd+S 保存")
                            .child("Cmd+Z/Y 撤销/重做")
                            .child("Ctrl+Space 切换 AI"),
                    ),
            )
            .child(
                div()
                    .id("editor-scroll")
                    .flex_1()
                    .w_full()
                    .rounded(px(8.0))
                    .bg(rgb(0x111111))
                    .border_1()
                    .border_color(rgb(0x222222))
                    .p_4()
                    .overflow_scroll()
                    .track_scroll(&self.scroll_handle)
                    .child({
                        let mut lines_container = div().flex().flex_col().gap_1();
                        for (idx, line) in self.lines.iter().enumerate() {
                            let is_cursor_line = cursor.map(|c| c.line == idx).unwrap_or(false);
                            let cursor_col = cursor.map(|c| c.column).unwrap_or(0);
                            let selection = self.selection;

                            let after = {
                                let chars: Vec<char> = line.chars().collect();
                                let split_at = std::cmp::min(cursor_col, chars.len());
                                chars.iter().skip(split_at).collect::<String>()
                            };

                            let mut row = div()
                                .flex()
                                .items_start()
                                .gap_3()
                                .w_full()
                                .bg(if is_cursor_line { rgb(0x181818) } else { rgb(0x111111) })
                                .px_2()
                                .py_1()
                                .rounded(px(4.0))
                                .id(("line", idx as u64))
                                .cursor_text()
                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(
                                    move |view: &mut EditorView, event: &gpui::MouseDownEvent, _, cx| {
                                        let col = view.hit_test_column(idx, event.position.x);
                                        view.dragging = true;
                                        view.set_cursor_position(idx, col, event.modifiers.shift, cx);
                                    },
                                ))
                                .on_mouse_move(cx.listener(
                                    move |view: &mut EditorView, event: &gpui::MouseMoveEvent, _, cx| {
                                        if view.dragging {
                                            let col = view.hit_test_column(idx, event.position.x);
                                            view.set_cursor_position(idx, col, true, cx);
                                            view.autoscroll_on_drag(event.position.y);
                                        }
                                    },
                                ))
                                .on_mouse_up(gpui::MouseButton::Left, cx.listener(
                                    move |view: &mut EditorView, event: &gpui::MouseUpEvent, _, cx| {
                                        if view.dragging {
                                            view.dragging = false;
                                            let col = view.hit_test_column(idx, event.position.x);
                                            view.set_cursor_position(idx, col, event.modifiers.shift, cx);
                                        }
                                    },
                                ));

                            row = row.child(
                                div()
                                    .w(px(48.0))
                                    .text_color(rgb(0x555555))
                                    .text_right()
                                    .child(format!("{:>4}", idx + 1)),
                            );

                            let mut line_content =
                                div().flex().gap_0().text_color(rgb(0xffffff));

                            // 选区高亮
                            if let Some(sel) = selection {
                                let start = sel.start();
                                let end = sel.end();
                                if idx >= start.line && idx <= end.line {
                                    let line_chars: Vec<char> = line.chars().collect();
                                    let len = line_chars.len();
                                    let range_start = if idx == start.line {
                                        start.column
                                    } else {
                                        0
                                    };
                                    let range_end = if idx == end.line {
                                        end.column.min(len)
                                    } else {
                                        len
                                    };

                                    if range_start < range_end {
                                        let before_sel: String =
                                            line_chars.iter().take(range_start).collect();
                                        let selected: String = line_chars
                                            .iter()
                                            .skip(range_start)
                                            .take(range_end - range_start)
                                            .collect();
                                        let after_sel: String =
                                            line_chars.iter().skip(range_end).collect();

                                        line_content = line_content
                                            .child(before_sel)
                                            .child(
                                                div()
                                                    .bg(rgb(0x223355))
                                                    .text_color(rgb(0xffffff))
                                                    .rounded(px(2.0))
                                                    .child(selected),
                                            )
                                            .child(after_sel);
                                    } else {
                                        line_content = line_content.child(line.clone());
                                    }
                                } else {
                                    line_content = line_content.child(line.clone());
                                }
                            } else {
                                line_content = line_content.child(line.clone());
                            }

                            // 光标
                            if is_cursor_line {
                                line_content = line_content
                                    .child(
                                        div()
                                            .w(px(2.0))
                                            .h(px(18.0))
                                            .bg(rgb(0x8ef1a2)),
                                    )
                                    .child(after);
                            }

                            row = row.child(line_content);
                            lines_container = lines_container.child(row);
                        }
                        lines_container
                    }),
            );

        content_area = content_area.child(editor_area);

        if self.show_ai_panel {
            if let Some(ai_panel) = &self.ai_panel {
                content_area = content_area.child(
                    div()
                        .w(px(380.0))
                        .flex()
                        .flex_col()
                        .bg(rgb(0x0b1627))
                        .border_l_1()
                        .border_color(rgb(0x1a2d4a))
                        .child(ai_panel.clone())
                        .child(
                            div()
                                .border_t_1()
                                .border_color(rgb(0x1a2d4a))
                                .p_3()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(div().text_color(rgb(0x9ecbff)).text_sm().child("Ask AI"))
                                .child(
                                    div()
                                        .id("ai-input")
                                        .rounded(px(6.0))
                                        .bg(if self.ai_input_focused {
                                            rgb(0x132d4b)
                                        } else {
                                            rgb(0x0f2038)
                                        })
                                        .border_1()
                                        .border_color(rgb(0x1a2d4a))
                                        .p_2()
                                        .cursor_text()
                                        .child(
                                            if self.ai_prompt_input.is_empty() {
                                                div()
                                                    .text_color(rgb(0x5f7a9c))
                                                    .child("输入问题，回车发送，Esc 退出")
                                            } else {
                                                div().text_color(rgb(0xd9e8ff)).child(
                                                    self.ai_prompt_input
                                                        .clone(),
                                                )
                                            },
                                        )
                                        .on_click(cx.listener(|view: &mut EditorView, _, _, cx| {
                                            view.ai_input_focused = true;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .id("ai-explain")
                                                .px_2()
                                                .py_1()
                                                .rounded(px(4.0))
                                                .bg(rgb(0x1a4d8f))
                                                .cursor_pointer()
                                                .text_sm()
                                                .child("解释当前文件")
                                                .on_click(cx.listener(
                                                    |view: &mut EditorView, _, _, cx| {
                                                        view.request_code_explanation(cx)
                                                    },
                                                )),
                                        )
                                        .child(
                                            div()
                                                .id("ai-improve")
                                                .px_2()
                                                .py_1()
                                                .rounded(px(4.0))
                                                .bg(rgb(0x1a4d8f))
                                                .cursor_pointer()
                                                .text_sm()
                                                .child("改进建议")
                                                .on_click(cx.listener(
                                                    |view: &mut EditorView, _, _, cx| {
                                                        view.request_code_improvements(cx)
                                                    },
                                                )),
                                        ),
                                ),
                        ),
                );
            }
        }

        layout
            .child(content_area)
            .child(
                div()
                    .h(px(28.0))
                    .px_3()
                    .bg(rgb(0x111111))
                    .border_t_1()
                    .border_color(rgb(0x2a2a2a))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_sm()
                    .text_color(rgb(0x888888))
                    .child(self.status_message.clone())
                    .child(format!(
                        "{} • UTC {}",
                        if self.is_dirty { "● 未保存" } else { "○ 已保存" },
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or_default()
                    )),
            )
            .child({
                if self.quick_open_active {
                    div()
                        .absolute()
                        .inset_0()
                        .bg(rgb(0x000000))
                        .opacity(0.6)
                        .child(
                            div()
                                .w(px(520.0))
                                .p_4()
                                .rounded(px(10.0))
                                .bg(rgb(0x121212))
                                .border_1()
                                .border_color(rgb(0x2a2a2a))
                                .shadow_lg()
                                .mx_auto()
                                .mt(px(120.0))
                                .child(div().text_color(rgb(0xffffff)).child("Quick Open"))
                                .child(
                                    div()
                                        .mt_2()
                                        .p_2()
                                        .rounded(px(6.0))
                                        .bg(rgb(0x0f0f0f))
                                        .border_1()
                                        .border_color(rgb(0x2a2a2a))
                                        .cursor_text()
                                        .child(self.quick_open_input.clone()),
                                )
                                .child(
                                    div()
                                        .mt_2()
                                        .text_sm()
                                        .text_color(rgb(0x888888))
                                        .child("输入相对路径，Enter 打开，Esc 取消"),
                                ),
                        )
                } else {
                    div()
                }
            })
    }
}

impl EditorView {
    pub fn handle_key_event(&mut self, event: &KeystrokeEvent, cx: &mut Context<'_, Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = &event.keystroke.modifiers;
        let command = modifiers.platform;

        // 快速打开模式下，按键只影响输入框
        if self.quick_open_active {
            match key {
                "Escape" => {
                    self.quick_open_active = false;
                    self.quick_open_input.clear();
                    cx.notify();
                }
                "Enter" => self.open_quick_input_path(cx),
                "Backspace" => {
                    self.quick_open_input.pop();
                    cx.notify();
                }
                _ if event.keystroke.key.len() == 1 => {
                    self.quick_open_input.push_str(&event.keystroke.key);
                    cx.notify();
                }
                _ => {}
            }
            return;
        }

        // AI 输入模式
        if self.ai_input_focused && self.show_ai_panel {
            match key {
                "Escape" => {
                    self.ai_input_focused = false;
                    cx.notify();
                }
                "Enter" => {
                    self.send_ai_prompt(cx);
                    self.ai_input_focused = false;
                }
                "Backspace" => self.backspace_ai_prompt(cx),
                _ if event.keystroke.key.len() == 1 => {
                    self.push_ai_prompt_char(&event.keystroke.key, cx);
                }
                _ => {}
            }
            return;
        }

        match key {
            "s" if command => self.save_current_file(cx),
            "o" if command => {
                self.quick_open_active = true;
                self.quick_open_input.clear();
                cx.notify();
            }
            "n" if command => self.new_buffer(cx),
            "p" if command && self.show_ai_panel => {
                self.ai_input_focused = true;
                cx.notify();
            }
            "z" if command => self.undo(cx),
            "y" if command => self.redo(cx),
            "f" if command => log::info!("Open find dialog"),
            "c" if command => self.copy_selection(cx),
            "v" if command => self.paste_text(cx),
            "/" if command => self.toggle_comment(cx),
            "]" if command => self.indent_code(cx),
            "[" if command => self.unindent_code(cx),
            " " if modifiers.control => self.toggle_ai_panel(cx),
            "ArrowLeft" | "Left" => self.move_cursor_by(CursorMovement::Left, modifiers.shift, cx),
            "ArrowRight" | "Right" => {
                self.move_cursor_by(CursorMovement::Right, modifiers.shift, cx)
            }
            "ArrowUp" | "Up" => self.move_cursor_by(CursorMovement::Up, modifiers.shift, cx),
            "ArrowDown" | "Down" => self.move_cursor_by(CursorMovement::Down, modifiers.shift, cx),
            "Home" => self.move_cursor_by(CursorMovement::Home, modifiers.shift, cx),
            "End" => self.move_cursor_by(CursorMovement::End, modifiers.shift, cx),
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
