use super::client::LspClient;
use super::protocol::{Diagnostic, Position};
use editor_infra::config::LSPServerConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug)]
pub struct LspServerManager {
    servers: Arc<RwLock<HashMap<String, Arc<Mutex<LspClient>>>>>,
    diagnostics: Arc<RwLock<HashMap<String, Vec<Diagnostic>>>>,
}

impl LspServerManager {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            diagnostics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_server_for_language(
        &self,
        config: &LSPServerConfig,
        workspace_root: &str,
    ) -> Result<(), std::io::Error> {
        let client = Arc::new(Mutex::new(LspClient::new()));
        {
            let mut client_guard = client.lock().await;
            client_guard
                .start_server(&config.command, &config.args)
                .await?;
            client_guard.initialize(workspace_root).await?;
        }

        let mut servers = self.servers.write().await;
        servers.insert(config.language.clone(), client);

        Ok(())
    }

    pub async fn get_server(&self, language: &str) -> Option<Arc<Mutex<LspClient>>> {
        let servers = self.servers.read().await;
        servers.get(language).cloned()
    }

    pub async fn request_completion(
        &self,
        language: &str,
        uri: &str,
        position: Position,
    ) -> Result<Vec<super::protocol::CompletionItem>, std::io::Error> {
        if let Some(client) = self.get_server(language).await {
            let mut client = client.lock().await;
            client.request_completion(uri, position).await
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn request_hover(
        &self,
        language: &str,
        uri: &str,
        position: Position,
    ) -> Result<Option<super::protocol::Hover>, std::io::Error> {
        if let Some(client) = self.get_server(language).await {
            let mut client = client.lock().await;
            client.request_hover(uri, position).await
        } else {
            Ok(None)
        }
    }

    pub async fn notify_file_opened(
        &self,
        language: &str,
        uri: &str,
        text: &str,
    ) -> Result<(), std::io::Error> {
        if let Some(client) = self.get_server(language).await {
            let mut client = client.lock().await;
            client.notify_did_open(uri, text, language).await
        } else {
            Ok(())
        }
    }

    pub async fn notify_file_changed(
        &self,
        language: &str,
        uri: &str,
        text: &str,
        version: u64,
    ) -> Result<(), std::io::Error> {
        if let Some(client) = self.get_server(language).await {
            let mut client = client.lock().await;
            client.notify_did_change(uri, text, version).await
        } else {
            Ok(())
        }
    }

    pub async fn update_diagnostics(&self, uri: String, diagnostics: Vec<Diagnostic>) {
        let mut current_diagnostics = self.diagnostics.write().await;
        current_diagnostics.insert(uri, diagnostics);
    }

    pub async fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        let current_diagnostics = self.diagnostics.read().await;
        current_diagnostics.get(uri).cloned().unwrap_or_default()
    }

    pub async fn shutdown_all(&self) -> Result<(), std::io::Error> {
        let mut servers = self.servers.write().await;
        for (_, client) in servers.drain() {
            let mut client = client.lock().await;
            client.shutdown().await?;
        }
        Ok(())
    }
}

impl Default for LspServerManager {
    fn default() -> Self {
        Self::new()
    }
}
