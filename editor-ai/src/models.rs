use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AIProvider {
    #[serde(rename = "openai_compatible")]
    OpenAICompatible,
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "azure_openai")]
    AzureOpenAI,
    #[serde(rename = "custom")]
    Custom,
}

impl AIProvider {
    pub fn as_str(&self) -> &str {
        match self {
            AIProvider::OpenAICompatible => "openai_compatible",
            AIProvider::Ollama => "ollama",
            AIProvider::Anthropic => "anthropic",
            AIProvider::AzureOpenAI => "azure_openai",
            AIProvider::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AIModel {
    #[serde(rename = "gpt-5")]
    GPT5,
    #[serde(rename = "gpt-4")]
    GPT4,
    #[serde(rename = "gpt-3.5-turbo")]
    GPT35Turbo,
    #[serde(rename = "claude-3")]
    Claude3,
    #[serde(rename = "codellama")]
    CodeLlama,
    #[serde(rename = "llama2")]
    Llama2,
    Custom(String),
}

impl AIModel {
    pub fn as_str(&self) -> &str {
        match self {
            AIModel::GPT5 => "gpt-5",
            AIModel::GPT4 => "gpt-4",
            AIModel::GPT35Turbo => "gpt-3.5-turbo",
            AIModel::Claude3 => "claude-3",
            AIModel::CodeLlama => "codellama",
            AIModel::Llama2 => "llama2",
            AIModel::Custom(s) => s,
        }
    }

    pub fn context_size(&self) -> usize {
        match self {
            AIModel::GPT5 => 128000,
            AIModel::GPT4 => 8192,
            AIModel::GPT35Turbo => 4096,
            AIModel::Claude3 => 200000,
            AIModel::CodeLlama => 16384,
            AIModel::Llama2 => 4096,
            AIModel::Custom(_) => 4096,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRequest {
    pub model: String,
    pub messages: Vec<AIMessage>,
    pub temperature: f32,
    pub max_tokens: Option<usize>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessage {
    pub role: AIRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AIRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

impl AIRole {
    pub fn as_str(&self) -> &str {
        match self {
            AIRole::System => "system",
            AIRole::User => "user",
            AIRole::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    pub id: Option<String>,
    pub choices: Vec<AIChoice>,
    pub usage: Option<AIUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIChoice {
    pub message: AIMessage,
    pub finish_reason: Option<String>,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: Option<PathBuf>,
    pub name: Option<String>,
    pub extension: Option<String>,
    pub language: String,
    pub line_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionInfo {
    pub text: String,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub is_multiline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorInfo {
    pub line: usize,
    pub column: usize,
    pub position_in_file: usize, // 字符位置
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub root_path: Option<PathBuf>,
    pub dependencies: Vec<String>,
    pub config_files: Vec<String>,
    pub related_files: Vec<FileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub timestamp: u64,
    pub context_size: usize,
    pub token_estimate: usize,
    pub language_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIContext {
    // 文件信息
    pub file_info: FileInfo,
    
    // 内容信息
    pub file_content: String,
    
    // 选择信息
    pub selection: Option<SelectionInfo>,
    
    // 光标信息
    pub cursor: CursorInfo,
    
    // 项目上下文
    pub project_context: Option<ProjectContext>,
    
    // 其他元数据
    pub metadata: ContextMetadata,
}

impl AIContext {
    pub fn new(
        file_content: String,
        file_info: FileInfo,
        cursor: CursorInfo,
    ) -> Self {
        Self {
            file_info,
            file_content,
            selection: None,
            cursor,
            project_context: None,
            metadata: ContextMetadata {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                context_size: 0,
                token_estimate: 0,
                language_features: Vec::new(),
            },
        }
    }

    pub fn with_selection(mut self, selection: SelectionInfo) -> Self {
        self.selection = Some(selection);
        self
    }

    pub fn with_project_context(mut self, context: ProjectContext) -> Self {
        self.project_context = Some(context);
        self
    }

    pub fn with_metadata(mut self, metadata: ContextMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    // 获取选中的文本（如果有）
    pub fn selected_text(&self) -> Option<&str> {
        self.selection.as_ref().map(|s| s.text.as_str())
    }

    // 获取文件路径
    pub fn file_path(&self) -> Option<&PathBuf> {
        self.file_info.path.as_ref()
    }

    // 计算上下文大小
    pub fn calculate_context_size(&mut self) -> usize {
        let mut size = self.file_content.len();
        if let Some(selection) = &self.selection {
            size += selection.text.len();
        }
        if let Some(project) = &self.project_context {
            size += project.dependencies.join(",").len();
            size += project.config_files.join(",").len();
        }
        self.metadata.context_size = size;
        size
    }

    // 转换为系统提示消息
    pub fn to_system_message(&self) -> AIMessage {
        let content = self.build_system_prompt();
        AIMessage {
            role: AIRole::System,
            content,
        }
    }

    fn build_system_prompt(&self) -> String {
        let mut prompt = String::new();

        // 添加项目上下文
        if let Some(project) = &self.project_context {
            prompt.push_str("## Project Context\n");
            if let Some(root) = &project.root_path {
                prompt.push_str(&format!("Root: {}\n", root.display()));
            }
            if !project.dependencies.is_empty() {
                prompt.push_str(&format!("Dependencies: {}\n", project.dependencies.join(", ")));
            }
            prompt.push_str("\n");
        }

        // 添加文件信息
        prompt.push_str("## Current File\n");
        prompt.push_str(&format!("Language: {}\n", self.file_info.language));
        if let Some(name) = &self.file_info.name {
            prompt.push_str(&format!("Name: {}\n", name));
        }
        if let Some(path) = &self.file_info.path {
            prompt.push_str(&format!("Path: {}\n", path.display()));
        }
        prompt.push_str(&format!("Lines: {}\n", self.file_info.line_count));
        prompt.push_str("\n");

        // 添加文件内容
        prompt.push_str("## File Content\n");
        prompt.push_str(&format!("```{}\n", self.file_info.language));
        prompt.push_str(&self.file_content);
        prompt.push_str("\n```\n\n");

        // 添加选区信息
        if let Some(selection) = &self.selection {
            prompt.push_str("## Selected Code\n");
            prompt.push_str(&format!("Position: L{}-C{} to L{}-C{}\n", 
                selection.start_line, selection.start_column,
                selection.end_line, selection.end_column));
            prompt.push_str(&format!("```{}\n", self.file_info.language));
            prompt.push_str(&selection.text);
            prompt.push_str("\n```\n\n");
        }

        // 添加光标位置
        prompt.push_str(&format!("## Cursor Position\nLine: {}, Column: {}\n\n", 
            self.cursor.line, self.cursor.column));

        prompt.push_str("You are an expert programming assistant. Provide helpful, accurate, and concise responses based on the code context provided.");

        prompt
    }
}

// 为缓冲区构建上下文的便捷方法
impl AIContext {
    pub async fn from_buffer(
        buffer: &editor_core_text::Buffer,
        file_path: Option<PathBuf>,
        language: String,
    ) -> anyhow::Result<Self> {
        let file_content = buffer.get_text().await;
        let line_count = buffer.line_count().await;
        
        let file_info = FileInfo {
            path: file_path.clone(),
            name: file_path.as_ref().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
            extension: file_path.as_ref().and_then(|p| p.extension().map(|e| e.to_string_lossy().to_string())),
            language,
            line_count,
        };

        let cursor_info = Self::get_cursor_info(buffer).await?;
        let selection_info = Self::get_selection_info(buffer).await?;

        let mut context = Self::new(file_content, file_info, cursor_info);
        
        if let Some(selection) = selection_info {
            context = context.with_selection(selection);
        }

        // 计算元数据
        context.calculate_context_size();
        // 这里可以添加 token 估算逻辑

        Ok(context)
    }

    async fn get_cursor_info(buffer: &editor_core_text::Buffer) -> anyhow::Result<CursorInfo> {
        let cursors = buffer.get_cursors();
        if let Some(cursor) = cursors.first() {
            // 计算字符位置（需要根据实际 Buffer 实现调整）
            let position_in_file = buffer.text_model.line_to_char(cursor.line).await + cursor.column;
            
            Ok(CursorInfo {
                line: cursor.line,
                column: cursor.column,
                position_in_file,
            })
        } else {
            Ok(CursorInfo {
                line: 0,
                column: 0,
                position_in_file: 0,
            })
        }
    }

    async fn get_selection_info(buffer: &editor_core_text::Buffer) -> anyhow::Result<Option<SelectionInfo>> {
        let selections = buffer.get_selections();
        if selections.is_empty() {
            return Ok(None);
        }

        let selection = &selections[0];
        if selection.is_collapsed() {
            return Ok(None);
        }

        let text = Self::extract_selection_text(buffer, selection).await?;
        
        Ok(Some(SelectionInfo {
            text,
            start_line: selection.start().line,
            start_column: selection.start().column,
            end_line: selection.end().line,
            end_column: selection.end().column,
            is_multiline: selection.start().line != selection.end().line,
        }))
    }

    async fn extract_selection_text(
        buffer: &editor_core_text::Buffer,
        selection: &editor_core_text::Selection,
    ) -> anyhow::Result<String> {
        // 实现选区文本提取逻辑
        // 这里需要根据实际的 Buffer API 来实现
        Ok("extracted_selection_text".to_string()) // 占位符
    }
}
