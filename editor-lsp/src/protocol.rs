use serde::{Deserialize, Serialize};
use serde_json::Value;

// 新增 LSP 方法枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LspMethod {
    #[serde(rename = "initialize")]
    Initialize,
    #[serde(rename = "textDocument/completion")]
    TextDocumentCompletion,
    #[serde(rename = "textDocument/hover")]
    TextDocumentHover,
    #[serde(rename = "textDocument/didOpen")]
    TextDocumentDidOpen,
    #[serde(rename = "textDocument/didChange")]
    TextDocumentDidChange,
    #[serde(rename = "textDocument/publishDiagnostics")]
    TextDocumentPublishDiagnostics,
    #[serde(rename = "shutdown")]
    Shutdown,
    #[serde(rename = "exit")]
    Exit,
    Custom(String),
}

impl LspMethod {
    pub fn as_str(&self) -> &str {
        match self {
            LspMethod::Initialize => "initialize",
            LspMethod::TextDocumentCompletion => "textDocument/completion",
            LspMethod::TextDocumentHover => "textDocument/hover",
            LspMethod::TextDocumentDidOpen => "textDocument/didOpen",
            LspMethod::TextDocumentDidChange => "textDocument/didChange",
            LspMethod::TextDocumentPublishDiagnostics => "textDocument/publishDiagnostics",
            LspMethod::Shutdown => "shutdown",
            LspMethod::Exit => "exit",
            LspMethod::Custom(s) => s,
        }
    }
}

impl From<LspMethod> for String {
    fn from(method: LspMethod) -> String {
        method.as_str().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<LspMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<LspError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRequest {
    pub id: u64,
    pub method: LspMethod,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspResponse {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<LspError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspNotification {
    pub method: LspMethod,
    pub params: Value,
}

// LSP Protocol specific structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

// 新增诊断严重性枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<DiagnosticSeverity>,
    pub code: Option<Value>,
    pub source: Option<String>,
    pub message: String,
}

// 新增完成项类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompletionItemKind {
    Text = 1,
    Method = 2,
    Function = 3,
    Constructor = 4,
    Field = 5,
    Variable = 6,
    Class = 7,
    Interface = 8,
    Module = 9,
    Property = 10,
    Unit = 11,
    Value = 12,
    Enum = 13,
    Keyword = 14,
    Snippet = 15,
    Color = 16,
    File = 17,
    Reference = 18,
    Folder = 19,
    EnumMember = 20,
    Constant = 21,
    Struct = 22,
    Event = 23,
    Operator = 24,
    TypeParameter = 25,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: Option<CompletionItemKind>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hover {
    pub contents: Value,
    pub range: Option<Range>,
}

impl LspMessage {
    pub fn new_request(id: u64, method: LspMethod, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: Some(method),
            params: Some(params),
            result: None,
            error: None,
        }
    }

    pub fn new_response(id: u64, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: None,
            params: None,
            result: Some(result),
            error: None,
        }
    }

    pub fn new_notification(method: LspMethod, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some(method),
            params: Some(params),
            result: None,
            error: None,
        }
    }

    pub fn is_request(&self) -> bool {
        self.id.is_some() && self.method.is_some()
    }

    pub fn is_response(&self) -> bool {
        self.id.is_some() && self.method.is_none()
    }

    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }
}
