use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditKind {
    Insert { char_idx: usize, text: String },
    Delete { char_idx: usize, text: String },
    Replace { char_idx: usize, old_text: String, new_text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edit {
    pub kind: EditKind,
    pub timestamp: SystemTime,
}

impl Edit {
    pub fn new_insert(char_idx: usize, text: String) -> Self {
        Self {
            kind: EditKind::Insert { char_idx, text },
            timestamp: SystemTime::now(),
        }
    }

    pub fn new_delete(char_idx: usize, text: String) -> Self {
        Self {
            kind: EditKind::Delete { char_idx, text },
            timestamp: SystemTime::now(),
        }
    }

    pub fn new_replace(char_idx: usize, old_text: String, new_text: String) -> Self {
        Self {
            kind: EditKind::Replace { char_idx, old_text, new_text },
            timestamp: SystemTime::now(),
        }
    }

    pub fn inverse(&self) -> Self {
        match &self.kind {
            EditKind::Insert { char_idx, text } => Self::new_delete(*char_idx, text.clone()),
            EditKind::Delete { char_idx, text } => Self::new_insert(*char_idx, text.clone()),
            EditKind::Replace { char_idx, old_text, new_text } => {
                Self::new_replace(*char_idx, new_text.clone(), old_text.clone())
            }
        }
    }

    pub fn description(&self) -> String {
        match &self.kind {
            EditKind::Insert { text, .. } => format!("Insert '{}'", if text.len() > 10 { &text[..10] } else { text }),
            EditKind::Delete { text, .. } => format!("Delete '{}'", if text.len() > 10 { &text[..10] } else { text }),
            EditKind::Replace { old_text, new_text, .. } => {
                format!("Replace '{}' with '{}'", 
                    if old_text.len() > 10 { &old_text[..10] } else { old_text },
                    if new_text.len() > 10 { &new_text[..10] } else { new_text }
                )
            }
        }
    }
}