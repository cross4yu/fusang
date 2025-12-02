use super::protocol::{CompletionItem, Hover, LspMessage, LspMethod, Position};
use serde_json::Value;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin as AsyncChildStdin, ChildStdout as AsyncChildStdout};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct LspClient {
    process: Option<Child>,
    stdin: Option<AsyncChildStdin>,
    stdout: Option<BufReader<AsyncChildStdout>>,
    next_request_id: u64,
    pending_requests: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<LspMessage>>>>,
}

impl LspClient {
    pub fn new() -> Self {
        Self {
            process: None,
            stdin: None,
            stdout: None,
            next_request_id: 1,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_server(
        &mut self,
        command: &str,
        args: &[String],
    ) -> Result<(), std::io::Error> {
        let mut command = Command::new(command);
        command.args(args);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn()?;

        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");

        let async_stdin = AsyncChildStdin::from_std(stdin)?;
        let async_stdout = AsyncChildStdout::from_std(stdout)?;

        self.stdin = Some(async_stdin);
        self.stdout = Some(BufReader::new(async_stdout));
        self.process = Some(child);

        // Start message processing loop
        self.start_message_processor().await;

        Ok(())
    }

    pub async fn initialize(&mut self, root_uri: &str) -> Result<Value, std::io::Error> {
        let initialize_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    }
                },
                "workspace": {
                    "configuration": true
                }
            },
            "trace": "off"
        });

        let response = self
            .send_request(LspMethod::Initialize, initialize_params)
            .await?;
        Ok(response)
    }

    pub async fn send_request(
        &mut self,
        method: LspMethod,
        params: Value,
    ) -> Result<Value, std::io::Error> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;

        let message = LspMessage::new_request(request_id, method, params);
        self.send_message(&message).await?;

        let (sender, receiver) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id, sender);
        }

        match receiver.await {
            Ok(response) => {
                if let Some(result) = response.result {
                    Ok(result)
                } else if let Some(error) = response.error {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("LSP error: {}", error.message),
                    ))
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Invalid LSP response",
                    ))
                }
            }
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Request timeout or channel closed",
            )),
        }
    }

    pub async fn send_notification(
        &mut self,
        method: LspMethod,
        params: Value,
    ) -> Result<(), std::io::Error> {
        let message = LspMessage::new_notification(method, params);
        self.send_message(&message).await
    }

    async fn send_message(&mut self, message: &LspMessage) -> Result<(), std::io::Error> {
        if let Some(stdin) = &mut self.stdin {
            let json = serde_json::to_string(message)?;
            let content = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
            stdin.write_all(content.as_bytes()).await?;
            stdin.flush().await?;
        }
        Ok(())
    }

    async fn start_message_processor(&mut self) {
        let stdout = match self.stdout.take() {
            Some(stdout) => stdout,
            None => return,
        };
        let pending_requests = self.pending_requests.clone();

        tokio::spawn(async move {
            let mut reader = stdout;
            let mut buffer = String::new();

            loop {
                buffer.clear();
                match reader.read_line(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if buffer.starts_with("Content-Length:") {
                            if let Some(content_length) = buffer
                                .strip_prefix("Content-Length:")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                            {
                                // Read the header separator
                                let mut separator = String::new();
                                if reader.read_line(&mut separator).await.is_ok()
                                    && separator == "\r\n"
                                {
                                    // Read the JSON content
                                    let mut content = vec![0u8; content_length];
                                    if reader.read_exact(&mut content).await.is_ok() {
                                        if let Ok(json_str) = String::from_utf8(content) {
                                            if let Ok(message) =
                                                serde_json::from_str::<LspMessage>(&json_str)
                                            {
                                                Self::handle_message(&message, &pending_requests)
                                                    .await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    async fn handle_message(
        message: &LspMessage,
        pending_requests: &Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<LspMessage>>>>,
    ) {
        if let Some(id) = message.id {
            let mut pending = pending_requests.lock().await;
            if let Some(sender) = pending.remove(&id) {
                let _ = sender.send(message.clone());
            }
        }
        // Handle notifications (like diagnostics) here
    }

    pub async fn request_completion(
        &mut self,
        uri: &str,
        position: Position,
    ) -> Result<Vec<CompletionItem>, std::io::Error> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": position
        });

        let result = self
            .send_request(LspMethod::TextDocumentCompletion, params)
            .await?;

        if let Some(items) = result.get("items").and_then(|i| i.as_array()) {
            let completions: Vec<CompletionItem> = items
                .iter()
                .filter_map(|item| serde_json::from_value(item.clone()).ok())
                .collect();
            Ok(completions)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn request_hover(
        &mut self,
        uri: &str,
        position: Position,
    ) -> Result<Option<Hover>, std::io::Error> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": position
        });

        let result = self
            .send_request(LspMethod::TextDocumentHover, params)
            .await?;
        serde_json::from_value(result)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub async fn notify_did_open(
        &mut self,
        uri: &str,
        text: &str,
        language_id: &str,
    ) -> Result<(), std::io::Error> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": 1,
                "text": text
            }
        });

        self.send_notification(LspMethod::TextDocumentDidOpen, params)
            .await
    }

    pub async fn notify_did_change(
        &mut self,
        uri: &str,
        text: &str,
        version: u64,
    ) -> Result<(), std::io::Error> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "version": version
            },
            "contentChanges": [{
                "text": text
            }]
        });

        self.send_notification(LspMethod::TextDocumentDidChange, params)
            .await
    }

    pub async fn shutdown(&mut self) -> Result<(), std::io::Error> {
        self.send_request(LspMethod::Shutdown, serde_json::Value::Null)
            .await?;
        self.send_notification(LspMethod::Exit, serde_json::Value::Null)
            .await?;

        if let Some(mut process) = self.process.take() {
            process.kill()?;
        }

        Ok(())
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}
