use ropey::Rope;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct TextModel {
    rope: Arc<RwLock<Rope>>,
    version: Arc<AtomicUsize>,
}

impl TextModel {
    pub fn new() -> Self {
        Self {
            rope: Arc::new(RwLock::new(Rope::new())),
            version: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Arc::new(RwLock::new(Rope::from_str(text))),
            version: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn get_text(&self) -> String {
        let rope = self.rope.read().await;
        rope.to_string()
    }

    pub async fn len(&self) -> usize {
        let rope = self.rope.read().await;
        rope.len_chars()
    }

    pub async fn is_empty(&self) -> bool {
        let rope = self.rope.read().await;
        rope.len_chars() == 0
    }

    pub async fn insert(&self, char_idx: usize, text: &str) {
        let mut rope = self.rope.write().await;
        
        if char_idx <= rope.len_chars() {
            rope.insert(char_idx, text);
            self.version.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub async fn remove(&self, char_idx: usize, len: usize) {
        let mut rope = self.rope.write().await;
        
        if char_idx < rope.len_chars() {
            let end_idx = (char_idx + len).min(rope.len_chars());
            rope.remove(char_idx..end_idx);
            self.version.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub async fn replace(&self, char_idx: usize, len: usize, text: &str) {
        let mut rope = self.rope.write().await;
        
        if char_idx < rope.len_chars() {
            let end_idx = (char_idx + len).min(rope.len_chars());
            rope.remove(char_idx..end_idx);
            rope.insert(char_idx, text);
            self.version.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub async fn get_char(&self, char_idx: usize) -> Option<char> {
        let rope = self.rope.read().await;
        rope.get_char(char_idx)
    }

    pub async fn get_line(&self, line_idx: usize) -> Option<String> {
        let rope = self.rope.read().await;
        if line_idx < rope.len_lines() {
            let line = rope.line(line_idx);
            Some(line.to_string())
        } else {
            None
        }
    }

    pub async fn line_count(&self) -> usize {
        let rope = self.rope.read().await;
        rope.len_lines()
    }

    pub async fn char_to_line(&self, char_idx: usize) -> usize {
        let rope = self.rope.read().await;
        rope.char_to_line(char_idx)
    }

    pub async fn line_to_char(&self, line_idx: usize) -> usize {
        let rope = self.rope.read().await;
        rope.line_to_char(line_idx)
    }

    pub fn version(&self) -> usize {
        self.version.load(Ordering::SeqCst)
    }
}

impl Default for TextModel {
    fn default() -> Self {
        Self::new()
    }
}