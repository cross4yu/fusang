use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIAction {
    GenerateCode {
        context: String,
        language: String,
        cursor_position: (usize, usize),
    },
    ExplainCode {
        code: String,
        language: String,
    },
    RefactorCode {
        code: String,
        language: String,
        refactoring_type: RefactoringType,
    },
    FixBugs {
        code: String,
        language: String,
        error_message: Option<String>,
    },
    GenerateTests {
        code: String,
        language: String,
        test_framework: String,
    },
    GenerateDocumentation {
        code: String,
        language: String,
    },
    Chat {
        message: String,
        conversation_history: Vec<ChatMessage>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringType {
    ExtractFunction,
    ExtractVariable,
    Rename,
    Simplify,
    Optimize,
    Cleanup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIPatch {
    pub file_path: String,
    pub old_code: String,
    pub new_code: String,
    pub description: String,
    pub line_range: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISuggestion {
    pub action: AIAction,
    pub confidence: f32,
    pub reasoning: String,
    pub alternatives: Vec<AIAction>,
}

impl AISuggestion {
    pub fn new(action: AIAction, confidence: f32, reasoning: String) -> Self {
        Self {
            action,
            confidence,
            reasoning,
            alternatives: Vec::new(),
        }
    }

    pub fn with_alternatives(mut self, alternatives: Vec<AIAction>) -> Self {
        self.alternatives = alternatives;
        self
    }

    pub fn is_high_confidence(&self) -> bool {
        self.confidence > 0.8
    }

    pub fn is_medium_confidence(&self) -> bool {
        self.confidence > 0.5 && self.confidence <= 0.8
    }

    pub fn is_low_confidence(&self) -> bool {
        self.confidence <= 0.5
    }
}

impl AIPatch {
    pub fn new(
        file_path: String,
        old_code: String,
        new_code: String,
        description: String,
        line_range: (usize, usize),
    ) -> Self {
        Self {
            file_path,
            old_code,
            new_code,
            description,
            line_range,
        }
    }

    pub fn apply(&self, current_code: &str) -> Option<String> {
        if current_code.contains(&self.old_code) {
            Some(current_code.replace(&self.old_code, &self.new_code))
        } else {
            None
        }
    }

    pub fn diff(&self) -> String {
        format!(
            "--- {}\n+++ {}\n@@ -{},{} +{},{} @@\n{}\n",
            self.file_path,
            self.file_path,
            self.line_range.0,
            self.line_range.1 - self.line_range.0,
            self.line_range.0,
            self.line_range.1 - self.line_range.0,
            self.description
        )
    }
}