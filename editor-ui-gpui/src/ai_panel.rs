use editor_ai::models::{AIContext, AIMessage, AIRole};
use editor_core_text::Buffer;
use gpui::{div, px, rgb, Context, Window, prelude::*};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AIPanel {
    messages: Vec<AIMessage>,
    current_model: String,
    is_loading: bool,
    ai_engine: Arc<editor_ai::AIEngine>,
    buffer_context: Option<AIContext>,
}

impl AIPanel {
    pub fn new(_cx: &mut Context<'_, Self>, ai_engine: Arc<editor_ai::AIEngine>) -> Self {
        Self {
            messages: Vec::new(),
            current_model: "gpt-3.5-turbo".to_string(),
            is_loading: false,
            ai_engine,
            buffer_context: None,
        }
    }

    /// 从缓冲区构建 AI 上下文
    pub async fn build_context_from_buffer(
        buffer: &Buffer,
        file_path: Option<PathBuf>,
        language: &str,
    ) -> anyhow::Result<AIContext> {
        AIContext::from_buffer(buffer, file_path, language.to_string()).await
    }

    /// 设置当前缓冲区上下文
    pub fn set_buffer_context(&mut self, context: AIContext) {
        self.buffer_context = Some(context);
    }

    /// 获取当前缓冲区上下文
    pub fn buffer_context(&self) -> Option<&AIContext> {
        self.buffer_context.as_ref()
    }

    /// 清除缓冲区上下文
    pub fn clear_buffer_context(&mut self) {
        self.buffer_context = None;
    }

    /// 发送消息到 AI
    pub async fn send_message(&mut self, message: String) -> anyhow::Result<()> {
        if self.is_loading {
            return Ok(());
        }

        self.is_loading = true;

        // 添加用户消息
        self.messages.push(AIMessage {
            role: AIRole::User,
            content: message.clone(),
        });

        // 如果有缓冲区上下文，构建完整的消息
        let mut messages_to_send = self.messages.clone();

        if let Some(context) = &self.buffer_context {
            // 构建包含上下文的系统消息
            let system_message = context.to_system_message();
            messages_to_send.insert(0, system_message);
        }

        // 发送到 AI 引擎
        let response = self
            .ai_engine
            .generate_chat_completion(messages_to_send, Some(&self.current_model))
            .await
            .map_err(|e| anyhow::anyhow!("AI engine error: {}", e))?;

        // 添加 AI 回复
        self.messages.push(AIMessage {
            role: AIRole::Assistant,
            content: response,
        });

        self.is_loading = false;
        Ok(())
    }

    /// 使用当前缓冲区上下文发送消息
    pub async fn send_message_with_context(&mut self, message: String) -> anyhow::Result<()> {
        if self.buffer_context.is_none() {
            return Err(anyhow::anyhow!("No buffer context available"));
        }
        self.send_message(message).await
    }

    /// 清除对话历史
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /// 获取消息列表
    pub fn messages(&self) -> &[AIMessage] {
        &self.messages
    }

    /// 获取消息列表的可变引用
    pub fn messages_mut(&mut self) -> &mut Vec<AIMessage> {
        &mut self.messages
    }

    /// 设置当前模型
    pub fn set_model(&mut self, model: String) {
        self.current_model = model;
    }

    /// 获取当前模型
    pub fn current_model(&self) -> &str {
        &self.current_model
    }

    /// 检查是否正在加载
    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    /// 获取可用的模型列表
    pub async fn get_available_models(&self) -> Vec<String> {
        self.ai_engine.get_available_models().await
    }

    /// 测试提供商连接
    pub async fn test_provider_connection(&self, provider_name: &str) -> anyhow::Result<bool> {
        self.ai_engine
            .test_provider_connection(provider_name)
            .await
            .map_err(|e| anyhow::anyhow!("Provider connection test failed: {}", e))
    }

    /// 更新 AI 配置
    pub async fn update_config(&self, new_config: editor_infra::config::AIConfig) {
        self.ai_engine.update_config(new_config).await;
    }

    /// 获取最后一条消息
    pub fn last_message(&self) -> Option<&AIMessage> {
        self.messages.last()
    }

    /// 获取消息数量
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// 检查是否有缓冲区上下文
    pub fn has_buffer_context(&self) -> bool {
        self.buffer_context.is_some()
    }

    /// 获取上下文摘要
    pub fn context_summary(&self) -> Option<String> {
        self.buffer_context.as_ref().map(|ctx| {
            let mut summary = format!("File: {}", ctx.file_info.language);
            if let Some(name) = &ctx.file_info.name {
                summary.push_str(&format!(" ({})", name));
            }
            if let Some(selection) = &ctx.selection {
                summary.push_str(&format!(", Selection: {} chars", selection.text.len()));
            }
            summary
        })
    }
}

impl Render for AIPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let mut layout = div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .text_sm()
            .bg(rgb(0x0b1627))
            .text_color(rgb(0xd9e8ff));

        layout = layout.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_color(rgb(0x8fd8ff)).child("AI Copilot"))
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded(px(6.0))
                        .bg(rgb(0x132c4d))
                        .text_xs()
                        .child(if self.is_loading {
                            "思考中…"
                        } else {
                            "空闲"
                        }),
                ),
        );

        if let Some(summary) = self.context_summary() {
            layout = layout.child(
                div()
                    .rounded(px(8.0))
                    .bg(rgb(0x132d4b))
                    .p_2()
                    .text_color(rgb(0xa6c8ff))
                    .child(format!("上下文: {}", summary)),
            );
        }

        let mut messages = div()
            .id("ai-messages")
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .rounded(px(8.0))
            .bg(rgb(0x0f2038))
            .border_1()
            .border_color(rgb(0x1a2d4a))
            .overflow_y_scroll();

        if self.messages.is_empty() {
            messages = messages.child(
                div()
                    .text_color(rgb(0x7ea6d6))
                    .child("暂无对话。使用 Ctrl+Space 打开面板后，可让它解释或改进当前文件。"),
            );
        } else {
            for message in &self.messages {
                let role_color = match message.role {
                    AIRole::User => rgb(0xb3f7a4),
                    AIRole::Assistant => rgb(0x9ecbff),
                    AIRole::System => rgb(0xffe4a6),
                };

                messages = messages.child(
                    div()
                        .rounded(px(6.0))
                        .bg(rgb(0x12223a))
                        .p_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(role_color)
                                .child(format!("{:?}", message.role)),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_color(rgb(0xd9e8ff))
                                .child(message.content.clone()),
                        ),
                );
            }
        }

        layout.child(messages)
    }
}

// 便捷方法扩展
impl AIPanel {
    /// 快速设置缓冲区上下文
    pub async fn set_buffer_context_from_buffer(
        &mut self,
        buffer: &Buffer,
        file_path: Option<PathBuf>,
        language: &str,
    ) -> anyhow::Result<()> {
        let context = Self::build_context_from_buffer(buffer, file_path, language).await?;
        self.set_buffer_context(context);
        Ok(())
    }

    /// 发送代码相关问题
    pub async fn ask_about_code(&mut self, question: &str) -> anyhow::Result<()> {
        let message = if self.has_buffer_context() {
            format!("关于当前代码：{}", question)
        } else {
            question.to_string()
        };
        self.send_message(message).await
    }

    /// 请求代码改进建议
    pub async fn request_code_improvements(&mut self) -> anyhow::Result<()> {
        let message =
            "请分析当前代码并提供改进建议，包括性能优化、代码风格、最佳实践等方面。".to_string();
        self.send_message_with_context(message).await
    }

    /// 请求代码解释
    pub async fn request_code_explanation(&mut self) -> anyhow::Result<()> {
        let message = "请解释当前代码的功能和工作原理。".to_string();
        self.send_message_with_context(message).await
    }
}
