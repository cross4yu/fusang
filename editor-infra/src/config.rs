use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub editor: EditorConfig,
    pub ai: AIConfig,
    pub lsp: LSPConfig,
    pub ui: UIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub tab_size: usize,
    pub use_spaces: bool,
    pub auto_save: bool,
    pub font_size: f32,
    pub font_family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub default_model: String,
    pub providers: HashMap<String, AIProviderConfig>,
    pub predefined_models: HashMap<String, PredefinedModelConfig>,
    pub model_groups: HashMap<String, ModelGroupConfig>,
    pub model_settings: HashMap<String, ModelSettings>,
    pub agents: HashMap<String, AgentConfig>,
    pub workflows: HashMap<String, WorkflowConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIProviderConfig {
    pub provider_type: AIProviderType,
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub enabled: bool,
    pub auto_discover: bool,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AIProviderType {
    #[serde(rename = "openai_compatible")]
    OpenAICompatible,
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "azure_openai")]
    AzureOpenAI,
    #[serde(rename = "google_vertex")]
    GoogleVertex,
    #[serde(rename = "huggingface")]
    HuggingFace,
    #[serde(rename = "replicate")]
    Replicate,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredefinedModelConfig {
    pub provider: String,
    pub model_name: String,
    pub display_name: String,
    pub description: String,
    pub context_size: usize,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub capabilities: ModelCapabilities,
    pub tags: Vec<String>,
    pub cost_per_1k_tokens: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub supports_chat: bool,
    pub supports_completion: bool,
    pub supports_vision: bool,
    pub supports_function_calling: bool,
    pub supports_streaming: bool,
    pub supports_embeddings: bool,
    pub supports_fine_tuning: bool,
    pub max_input_tokens: Option<usize>,
    pub max_output_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelGroupConfig {
    pub name: String,
    pub description: String,
    pub models: Vec<String>,
    pub default_model: String,
    pub use_case: ModelUseCase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelUseCase {
    #[serde(rename = "code_completion")]
    CodeCompletion,
    #[serde(rename = "chat")]
    Chat,
    #[serde(rename = "refactoring")]
    Refactoring,
    #[serde(rename = "documentation")]
    Documentation,
    #[serde(rename = "debugging")]
    Debugging,
    #[serde(rename = "testing")]
    Testing,
    #[serde(rename = "code_review")]
    CodeReview,
    #[serde(rename = "optimization")]
    Optimization,
    #[serde(rename = "general")]
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    pub temperature: f32,
    pub max_tokens: usize,
    pub top_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    pub stop_sequences: Vec<String>,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub description: String,
    pub model: String,
    pub system_prompt: String,
    pub capabilities: Vec<AgentCapability>,
    pub tools: Vec<String>,
    pub temperature: f32,
    pub max_tokens: usize,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentCapability {
    #[serde(rename = "code_generation")]
    CodeGeneration,
    #[serde(rename = "code_explanation")]
    CodeExplanation,
    #[serde(rename = "bug_fixing")]
    BugFixing,
    #[serde(rename = "refactoring")]
    Refactoring,
    #[serde(rename = "testing")]
    Testing,
    #[serde(rename = "documentation")]
    Documentation,
    #[serde(rename = "code_review")]
    CodeReview,
    #[serde(rename = "optimization")]
    Optimization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub triggers: Vec<WorkflowTrigger>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub agent: String,
    pub input_template: String,
    pub output_handling: OutputHandling,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputHandling {
    #[serde(rename = "replace")]
    Replace,
    #[serde(rename = "append")]
    Append,
    #[serde(rename = "create_new")]
    CreateNew,
    #[serde(rename = "ignore")]
    Ignore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowTrigger {
    #[serde(rename = "file_saved")]
    FileSaved,
    #[serde(rename = "file_opened")]
    FileOpened,
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "timer")]
    Timer { interval_seconds: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPConfig {
    pub enabled: bool,
    pub servers: Vec<LSPServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPServerConfig {
    pub language: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub theme: String,
    pub show_line_numbers: bool,
    pub show_minimap: bool,
}

// 运行时模型信息
#[derive(Debug, Clone)]
pub struct DiscoveredModel {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub provider: String,
    pub provider_type: AIProviderType,
    pub context_size: usize,
    pub capabilities: ModelCapabilities,
    pub is_local: bool,
    pub is_available: bool,
    pub last_used: Option<std::time::SystemTime>,
    pub usage_count: u64,
    pub average_response_time: Option<f64>,
    pub tags: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        // 创建默认的 provider
        let mut providers = HashMap::new();
        providers.insert(
            "local-ollama".to_string(),
            AIProviderConfig {
                provider_type: AIProviderType::Ollama,
                base_url: "http://localhost:11434".to_string(),
                api_key: None,
                timeout_seconds: Some(30),
                enabled: true,
                auto_discover: true,
                priority: 10,
            },
        );
        providers.insert(
            "openai".to_string(),
            AIProviderConfig {
                provider_type: AIProviderType::OpenAICompatible,
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: None,
                timeout_seconds: Some(60),
                enabled: false,
                auto_discover: false,
                priority: 5,
            },
        );

        // 创建预定义的模型配置，包括 GPT-5
        let mut predefined_models = HashMap::new();

        // GPT-5 模型配置
        predefined_models.insert(
            "gpt-5".to_string(),
            PredefinedModelConfig {
                provider: "openai".to_string(),
                model_name: "gpt-5".to_string(),
                display_name: "GPT-5".to_string(),
                description:
                    "OpenAI's next-generation model with enhanced capabilities and larger context"
                        .to_string(),
                context_size: 128000,
                max_tokens: Some(16384),
                temperature: Some(0.7),
                top_p: Some(1.0),
                frequency_penalty: Some(0.0),
                presence_penalty: Some(0.0),
                stop_sequences: vec!["\n\n".to_string()],
                capabilities: ModelCapabilities {
                    supports_chat: true,
                    supports_completion: true,
                    supports_vision: true,
                    supports_function_calling: true,
                    supports_streaming: true,
                    supports_embeddings: true,
                    supports_fine_tuning: false,
                    max_input_tokens: Some(128000),
                    max_output_tokens: Some(16384),
                },
                tags: vec![
                    "general".to_string(),
                    "chat".to_string(),
                    "vision".to_string(),
                    "latest".to_string(),
                ],
                cost_per_1k_tokens: Some(0.05),
            },
        );

        predefined_models.insert(
            "gpt-4".to_string(),
            PredefinedModelConfig {
                provider: "openai".to_string(),
                model_name: "gpt-4".to_string(),
                display_name: "GPT-4".to_string(),
                description: "OpenAI's most capable model".to_string(),
                context_size: 8192,
                max_tokens: Some(4096),
                temperature: Some(0.7),
                top_p: Some(1.0),
                frequency_penalty: Some(0.0),
                presence_penalty: Some(0.0),
                stop_sequences: vec!["\n\n".to_string()],
                capabilities: ModelCapabilities {
                    supports_chat: true,
                    supports_completion: true,
                    supports_vision: false,
                    supports_function_calling: true,
                    supports_streaming: true,
                    supports_embeddings: false,
                    supports_fine_tuning: false,
                    max_input_tokens: Some(8192),
                    max_output_tokens: Some(4096),
                },
                tags: vec!["general".to_string(), "chat".to_string()],
                cost_per_1k_tokens: Some(0.03),
            },
        );

        predefined_models.insert(
            "gpt-3.5-turbo".to_string(),
            PredefinedModelConfig {
                provider: "openai".to_string(),
                model_name: "gpt-3.5-turbo".to_string(),
                display_name: "GPT-3.5 Turbo".to_string(),
                description: "OpenAI's fast and cost-effective model".to_string(),
                context_size: 4096,
                max_tokens: Some(2048),
                temperature: Some(0.7),
                top_p: Some(1.0),
                frequency_penalty: Some(0.0),
                presence_penalty: Some(0.0),
                stop_sequences: vec!["\n\n".to_string()],
                capabilities: ModelCapabilities {
                    supports_chat: true,
                    supports_completion: true,
                    supports_vision: false,
                    supports_function_calling: true,
                    supports_streaming: true,
                    supports_embeddings: false,
                    supports_fine_tuning: false,
                    max_input_tokens: Some(4096),
                    max_output_tokens: Some(2048),
                },
                tags: vec![
                    "general".to_string(),
                    "chat".to_string(),
                    "fast".to_string(),
                ],
                cost_per_1k_tokens: Some(0.002),
            },
        );

        // 创建模型组
        let mut model_groups = HashMap::new();
        model_groups.insert(
            "latest-models".to_string(),
            ModelGroupConfig {
                name: "Latest Models".to_string(),
                description: "Most recent and advanced AI models".to_string(),
                models: vec!["gpt-5".to_string(), "gpt-4".to_string()],
                default_model: "gpt-5".to_string(),
                use_case: ModelUseCase::General,
            },
        );
        model_groups.insert(
            "code-completion".to_string(),
            ModelGroupConfig {
                name: "Code Completion".to_string(),
                description: "Models optimized for code completion".to_string(),
                models: vec!["codellama".to_string(), "starcoder".to_string()],
                default_model: "codellama".to_string(),
                use_case: ModelUseCase::CodeCompletion,
            },
        );

        // 创建默认的 agent
        let mut agents = HashMap::new();
        agents.insert(
            "code-assistant".to_string(),
            AgentConfig {
                name: "Code Assistant".to_string(),
                description: "General purpose code assistance".to_string(),
                model: "gpt-5".to_string(),
                system_prompt: "You are a helpful coding assistant. Provide clear, concise code suggestions and explanations.".to_string(),
                capabilities: vec![
                    AgentCapability::CodeGeneration,
                    AgentCapability::CodeExplanation,
                    AgentCapability::BugFixing,
                ],
                tools: vec!["editor".to_string(), "filesystem".to_string()],
                temperature: 0.7,
                max_tokens: 2000,
                enabled: true,
            },
        );

        // 创建默认的 workflow
        let mut workflows = HashMap::new();
        workflows.insert(
            "auto-documentation".to_string(),
            WorkflowConfig {
                name: "Auto Documentation".to_string(),
                description: "Automatically generate documentation for code".to_string(),
                steps: vec![
                    WorkflowStep {
                        name: "analyze-code".to_string(),
                        agent: "code-assistant".to_string(),
                        input_template: "Analyze the following code and generate comprehensive documentation:\n\n{{code}}".to_string(),
                        output_handling: OutputHandling::CreateNew,
                        conditions: vec!["is_function".to_string()],
                    },
                ],
                triggers: vec![WorkflowTrigger::Manual],
                enabled: true,
            },
        );

        Self {
            editor: EditorConfig {
                tab_size: 4,
                use_spaces: true,
                auto_save: false,
                font_size: 14.0,
                font_family: "Monaco".to_string(),
            },
            ai: AIConfig {
                default_model: "gpt-5".to_string(),
                providers,
                predefined_models,
                model_groups,
                model_settings: HashMap::new(),
                agents,
                workflows,
            },
            lsp: LSPConfig {
                enabled: true,
                servers: vec![
                    LSPServerConfig {
                        language: "rust".to_string(),
                        command: "rust-analyzer".to_string(),
                        args: vec![],
                    },
                    LSPServerConfig {
                        language: "python".to_string(),
                        command: "pylsp".to_string(),
                        args: vec![],
                    },
                ],
            },
            ui: UIConfig {
                theme: "dark".to_string(),
                show_line_numbers: true,
                show_minimap: true,
            },
        }
    }
}

impl Config {
    pub fn load_from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    // 获取启用的 provider 列表
    pub fn get_enabled_providers(&self) -> Vec<&AIProviderConfig> {
        self.ai.providers.values().filter(|p| p.enabled).collect()
    }

    // 获取预定义的模型配置
    pub fn get_predefined_model(&self, model_name: &str) -> Option<&PredefinedModelConfig> {
        self.ai.predefined_models.get(model_name)
    }

    // 获取所有预定义模型
    pub fn get_all_predefined_models(&self) -> Vec<&PredefinedModelConfig> {
        self.ai.predefined_models.values().collect()
    }

    // 获取启用的 agent
    pub fn get_enabled_agents(&self) -> Vec<&AgentConfig> {
        self.ai.agents.values().filter(|a| a.enabled).collect()
    }

    // 获取启用的 workflow
    pub fn get_enabled_workflows(&self) -> Vec<&WorkflowConfig> {
        self.ai.workflows.values().filter(|w| w.enabled).collect()
    }
}
