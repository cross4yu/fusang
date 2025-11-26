use super::models::{AIContext, AIMessage, AIRequest, AIResponse, AIRole};
use editor_infra::config::{AIConfig, AIProviderConfig, PredefinedModelConfig};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum AIEngineError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("JSON serialization failed: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("API key required but not provided")]
    ApiKeyRequired,
    #[error("Request timeout")]
    Timeout,
}

#[derive(Debug, Clone)]
pub struct AIEngine {
    config: Arc<RwLock<AIConfig>>,
    http_client: Client,
    model_cache: Arc<RwLock<HashMap<String, PredefinedModelConfig>>>,
}

impl AIEngine {
    pub fn new(config: AIConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            http_client: Client::new(),
            model_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn generate_completion(
        &self,
        context: AIContext,
        model_name: Option<&str>,
    ) -> Result<String, AIEngineError> {
        let owned_model_name;
        let model_name = if let Some(name) = model_name {
            name
        } else {
            owned_model_name = {
                let cfg = self.config.read().await;
                cfg.default_model.clone()
            };
            &owned_model_name
        };

        let model_config = self.get_model_config(model_name).await?;
        let provider_config = self.get_provider_config(&model_config.provider).await?;

        let messages = self.build_messages(context, &model_config).await?;

        let request = AIRequest {
            model: model_config.model_name.clone(),
            messages,
            temperature: model_config.temperature.unwrap_or(0.7),
            max_tokens: model_config.max_tokens,
            stream: false,
        };

        let response = self.send_request(&provider_config, &request).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(AIEngineError::ConfigError(
                "No response from AI".to_string(),
            ))
        }
    }

    pub async fn generate_chat_completion(
        &self,
        messages: Vec<AIMessage>,
        model_name: Option<&str>,
    ) -> Result<String, AIEngineError> {
        let owned_model_name;
        let model_name = if let Some(name) = model_name {
            name
        } else {
            owned_model_name = {
                let cfg = self.config.read().await;
                cfg.default_model.clone()
            };
            &owned_model_name
        };

        let model_config = self.get_model_config(model_name).await?;
        let provider_config = self.get_provider_config(&model_config.provider).await?;

        let request = AIRequest {
            model: model_config.model_name.clone(),
            messages,
            temperature: model_config.temperature.unwrap_or(0.7),
            max_tokens: model_config.max_tokens,
            stream: false,
        };

        let response = self.send_request(&provider_config, &request).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(AIEngineError::ConfigError(
                "No response from AI".to_string(),
            ))
        }
    }

    async fn build_messages(
        &self,
        context: AIContext,
        model_config: &PredefinedModelConfig,
    ) -> Result<Vec<AIMessage>, AIEngineError> {
        let mut messages = Vec::new();

        // 使用新的 AIContext 方法构建系统消息
        let system_message = context.to_system_message();
        messages.push(system_message);

        // 构建用户消息
        let user_message = self.build_user_message(context, model_config).await;
        messages.push(AIMessage {
            role: AIRole::User,
            content: user_message,
        });

        Ok(messages)
    }

    async fn build_user_message(
        &self,
        context: AIContext,
        model_config: &PredefinedModelConfig,
    ) -> String {
        let mut message = String::new();

        // 添加项目上下文（如果有）
        if let Some(project_context) = &context.project_context {
            message.push_str("## Project Context\n");
            if let Some(root_path) = &project_context.root_path {
                message.push_str(&format!("Root: {}\n", root_path.display()));
            }
            if !project_context.dependencies.is_empty() {
                message.push_str(&format!(
                    "Dependencies: {}\n",
                    project_context.dependencies.join(", ")
                ));
            }
            message.push_str("\n");
        }

        // 添加文件信息
        message.push_str(&format!(
            "## Current File\nLanguage: {}\n",
            context.file_info.language
        ));
        if let Some(name) = &context.file_info.name {
            message.push_str(&format!("Name: {}\n", name));
        }
        if let Some(path) = &context.file_info.path {
            message.push_str(&format!("Path: {}\n", path.display()));
        }
        message.push_str(&format!("Lines: {}\n\n", context.file_info.line_count));

        // 添加文件内容
        message.push_str("## File Content\n");
        message.push_str(&format!(
            "```{}\n{}\n```\n\n",
            context.file_info.language, context.file_content
        ));

        // 添加选区内容（如果有）
        if let Some(selection) = &context.selection {
            message.push_str("## Selected Code\n");
            message.push_str(&format!(
                "Position: L{}-C{} to L{}-C{}\n",
                selection.start_line,
                selection.start_column,
                selection.end_line,
                selection.end_column
            ));
            message.push_str(&format!(
                "```{}\n{}\n```\n\n",
                context.file_info.language, selection.text
            ));
        }

        // 添加光标位置信息
        message.push_str(&format!(
            "## Cursor Position\nLine: {}, Column: {}\n\n",
            context.cursor.line, context.cursor.column
        ));

        message.push_str("Please provide helpful code suggestions, explanations, or improvements based on the above context.");

        message
    }

    async fn get_system_prompt(&self, language: &str) -> Option<String> {
        match language {
            "rust" => Some("You are an expert Rust programmer. Provide safe, efficient, and idiomatic Rust code.".to_string()),
            "python" => Some("You are an expert Python programmer. Provide clean, readable, and Pythonic code.".to_string()),
            "javascript" | "typescript" => Some("You are an expert JavaScript/TypeScript programmer. Provide modern, efficient, and well-typed code.".to_string()),
            _ => Some("You are an expert programmer. Provide clear, concise, and well-structured code.".to_string()),
        }
    }

    async fn send_request(
        &self,
        provider_config: &AIProviderConfig,
        request: &AIRequest,
    ) -> Result<AIResponse, AIEngineError> {
        let url = match provider_config.provider_type {
            editor_infra::config::AIProviderType::Ollama => {
                format!("{}/api/chat", provider_config.base_url)
            }
            _ => {
                format!("{}/chat/completions", provider_config.base_url)
            }
        };

        let mut http_request = self.http_client.post(&url).json(request);

        // 添加 API key（如果需要）
        if let Some(api_key) = &provider_config.api_key {
            http_request = http_request.header("Authorization", format!("Bearer {}", api_key));
        }

        // 设置超时
        if let Some(timeout) = provider_config.timeout_seconds {
            http_request = http_request.timeout(std::time::Duration::from_secs(timeout));
        }

        let response = http_request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AIEngineError::ConfigError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let ai_response: AIResponse = response.json().await?;
        Ok(ai_response)
    }

    async fn get_model_config(
        &self,
        model_name: &str,
    ) -> Result<PredefinedModelConfig, AIEngineError> {
        let config = self.config.read().await;

        if let Some(model_config) = config.predefined_models.get(model_name) {
            Ok(model_config.clone())
        } else {
            Err(AIEngineError::ModelNotFound(model_name.to_string()))
        }
    }

    async fn get_provider_config(
        &self,
        provider_name: &str,
    ) -> Result<AIProviderConfig, AIEngineError> {
        let config = self.config.read().await;

        if let Some(provider_config) = config.providers.get(provider_name) {
            if !provider_config.enabled {
                return Err(AIEngineError::ConfigError(format!(
                    "Provider '{}' is disabled",
                    provider_name
                )));
            }
            Ok(provider_config.clone())
        } else {
            Err(AIEngineError::ProviderNotFound(provider_name.to_string()))
        }
    }

    pub async fn update_config(&self, new_config: AIConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
    }

    pub async fn get_available_models(&self) -> Vec<String> {
        let config = self.config.read().await;
        config.predefined_models.keys().cloned().collect()
    }

    pub async fn test_provider_connection(
        &self,
        provider_name: &str,
    ) -> Result<bool, AIEngineError> {
        let provider_config = self.get_provider_config(provider_name).await?;

        // 简单的连接测试：发送一个空的请求或模型列表请求
        let url = match provider_config.provider_type {
            editor_infra::config::AIProviderType::Ollama => {
                format!("{}/api/tags", provider_config.base_url)
            }
            _ => {
                // 对于其他提供商，暂时返回成功
                return Ok(true);
            }
        };

        let response = self.http_client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}
