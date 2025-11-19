use editor_core_text::Buffer;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct BufferManager {
    buffers: Arc<RwLock<HashMap<PathBuf, Buffer>>>,
    current_buffer: Arc<RwLock<Option<PathBuf>>>,
}

impl BufferManager {
    pub fn new() -> Self {
        Self {
            buffers: Arc::new(RwLock::new(HashMap::new())),
            current_buffer: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn open_file(&self, file_path: &Path) -> Result<(), std::io::Error> {
        let content = std::fs::read_to_string(file_path)?;
        let buffer = Buffer::from_text(&content);
        
        let mut buffers = self.buffers.write().await;
        buffers.insert(file_path.to_path_buf(), buffer);
        
        let mut current = self.current_buffer.write().await;
        *current = Some(file_path.to_path_buf());
        
        Ok(())
    }

    pub async fn create_new_buffer(&self) -> PathBuf {
        let temp_path = PathBuf::from(format!("untitled-{}", uuid::Uuid::new_v4()));
        let buffer = Buffer::new();
        
        let mut buffers = self.buffers.write().await;
        buffers.insert(temp_path.clone(), buffer);
        
        let mut current = self.current_buffer.write().await;
        *current = Some(temp_path.clone());
        
        temp_path
    }

    pub async fn save_file(&self, file_path: &Path) -> Result<(), std::io::Error> {
        let buffers = self.buffers.read().await;
        if let Some(buffer) = buffers.get(file_path) {
            let content = buffer.get_text().await;
            std::fs::write(file_path, content)?;
            
            // Mark buffer as clean after saving
            // Note: We need to get mutable access to mark it clean
            drop(buffers); // Release read lock
            let mut buffers = self.buffers.write().await;
            if let Some(buffer) = buffers.get_mut(file_path) {
                buffer.mark_clean();
            }
        }
        Ok(())
    }

    pub async fn save_current_file(&self) -> Result<(), std::io::Error> {
        let current = self.current_buffer.read().await;
        if let Some(path) = &*current {
            self.save_file(path).await
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No current buffer to save",
            ))
        }
    }

    pub async fn close_file(&self, file_path: &Path) -> Result<(), std::io::Error> {
        let mut buffers = self.buffers.write().await;
        buffers.remove(file_path);
        
        let mut current = self.current_buffer.write().await;
        if current.as_ref() == Some(&file_path.to_path_buf()) {
            *current = buffers.keys().next().cloned();
        }
        
        Ok(())
    }

    pub async fn get_current_buffer(&self) -> Option<Buffer> {
        let current = self.current_buffer.read().await;
        let buffers = self.buffers.read().await;
        
        current.as_ref().and_then(|path| buffers.get(path)).cloned()
    }

    pub async fn set_current_buffer(&self, file_path: &Path) -> Result<(), std::io::Error> {
        let buffers = self.buffers.read().await;
        if buffers.contains_key(file_path) {
            let mut current = self.current_buffer.write().await;
            *current = Some(file_path.to_path_buf());
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Buffer not found",
            ))
        }
    }

    pub async fn get_buffer(&self, file_path: &Path) -> Option<Buffer> {
        let buffers = self.buffers.read().await;
        buffers.get(file_path).cloned()
    }

    pub async fn has_unsaved_changes(&self) -> bool {
        let buffers = self.buffers.read().await;
        buffers.values().any(|buffer| buffer.is_dirty())
    }

    pub async fn get_unsaved_files(&self) -> Vec<PathBuf> {
        let buffers = self.buffers.read().await;
        buffers
            .iter()
            .filter(|(_, buffer)| buffer.is_dirty())
            .map(|(path, _)| path.clone())
            .collect()
    }

    pub async fn get_open_files(&self) -> Vec<PathBuf> {
        let buffers = self.buffers.read().await;
        buffers.keys().cloned().collect()
    }

    pub async fn get_current_file_path(&self) -> Option<PathBuf> {
        let current = self.current_buffer.read().await;
        current.clone()
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}