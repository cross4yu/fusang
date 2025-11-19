use super::{cursor::Cursor, selection::Selection, text_model::TextModel};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Buffer {
    text_model: Arc<TextModel>,
    cursors: Vec<Cursor>,
    selections: Vec<Selection>,
    is_dirty: bool,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            text_model: Arc::new(TextModel::new()),
            cursors: vec![Cursor::zero()],
            selections: vec![Selection::single(Cursor::zero())],
            is_dirty: false,
        }
    }

    pub fn from_text(text: &str) -> Self {
        Self {
            text_model: Arc::new(TextModel::from_str(text)),
            cursors: vec![Cursor::zero()],
            selections: vec![Selection::single(Cursor::zero())],
            is_dirty: false,
        }
    }

    pub async fn get_text(&self) -> String {
        self.text_model.get_text().await
    }

    pub async fn insert_text_at_cursor(&mut self, text: &str) {
        if self.selections.is_empty() {
            return;
        }

        struct SelectionEdit {
            index: usize,
            start_char_idx: usize,
            end_char_idx: usize,
            collapsed: bool,
        }

        // Collect all edits before mutating the rope so char indices stay valid
        let mut edits = Vec::with_capacity(self.selections.len());
        for (index, selection) in self.selections.iter().enumerate() {
            let collapsed = selection.is_collapsed();
            let start = selection.start();
            let start_char_idx = self.text_model.line_to_char(start.line).await + start.column;
            let end_char_idx = if collapsed {
                start_char_idx
            } else {
                let end = selection.end();
                self.text_model.line_to_char(end.line).await + end.column
            };

            edits.push(SelectionEdit {
                index,
                start_char_idx,
                end_char_idx,
                collapsed,
            });
        }

        let touched_indices: Vec<usize> = edits.iter().map(|edit| edit.index).collect();

        // Apply edits from the end of the buffer to avoid adjusting subsequent char indices
        edits.sort_by(|a, b| b.start_char_idx.cmp(&a.start_char_idx));
        for edit in &edits {
            if edit.collapsed {
                self.text_model.insert(edit.start_char_idx, text).await;
            } else {
                let length = edit.end_char_idx.saturating_sub(edit.start_char_idx);
                self.text_model.replace(edit.start_char_idx, length, text).await;
            }
        }

        self.is_dirty = true;
        self.update_cursors_after_insert(&touched_indices, text);
    }

    pub async fn insert_text_at_position(&mut self, line: usize, column: usize, text: &str) {
        let char_idx = self.text_model.line_to_char(line).await + column;
        self.text_model.insert(char_idx, text).await;
        self.is_dirty = true;
        
        // Update cursor to end of inserted text
        let new_column = column + text.chars().count();
        self.set_cursor(Cursor::new(line, new_column));
    }

    pub async fn insert_line_break(&mut self) {
        self.insert_text_at_cursor("\n").await;
    }

    pub async fn insert_tab(&mut self, tab_size: usize) {
        let spaces = " ".repeat(tab_size);
        self.insert_text_at_cursor(&spaces).await;
    }

    pub async fn delete_backward(&mut self) {
        if self.selections.is_empty() {
            return;
        }

        struct DeleteEdit {
            index: usize,
            start_char_idx: usize,
            len: usize,
            new_cursor: Cursor,
        }

        let mut edits = Vec::with_capacity(self.selections.len());

        for (index, selection) in self.selections.iter().enumerate() {
            if selection.is_collapsed() {
                let cursor = selection.active;
                if cursor.column > 0 {
                    let line_offset = self.text_model.line_to_char(cursor.line).await;
                    let char_idx = line_offset + cursor.column;
                    edits.push(DeleteEdit {
                        index,
                        start_char_idx: char_idx - 1,
                        len: 1,
                        new_cursor: Cursor::new(cursor.line, cursor.column - 1),
                    });
                } else if cursor.line > 0 {
                    let prev_line_length = self.text_model.get_line(cursor.line - 1).await
                        .map(|line| line.chars().count())
                        .unwrap_or(0);
                    let line_start = self.text_model.line_to_char(cursor.line).await;
                    if line_start > 0 {
                        edits.push(DeleteEdit {
                            index,
                            start_char_idx: line_start - 1,
                            len: 1,
                            new_cursor: Cursor::new(cursor.line - 1, prev_line_length),
                        });
                    }
                }
            } else {
                let start = selection.start();
                let end = selection.end();
                let start_char_idx = self.text_model.line_to_char(start.line).await + start.column;
                let end_char_idx = self.text_model.line_to_char(end.line).await + end.column;
                if end_char_idx > start_char_idx {
                    edits.push(DeleteEdit {
                        index,
                        start_char_idx,
                        len: end_char_idx - start_char_idx,
                        new_cursor: start,
                    });
                }
            }
        }

        if edits.is_empty() {
            return;
        }

        edits.sort_by(|a, b| b.start_char_idx.cmp(&a.start_char_idx));
        for edit in &edits {
            self.text_model.remove(edit.start_char_idx, edit.len).await;
        }
        self.is_dirty = true;

        for edit in edits {
            if let Some(cursor_slot) = self.cursors.get_mut(edit.index) {
                *cursor_slot = edit.new_cursor;
            }
            if let Some(selection_slot) = self.selections.get_mut(edit.index) {
                *selection_slot = Selection::single(edit.new_cursor);
            }
        }
    }

    pub async fn delete_forward(&mut self) {
        if self.selections.is_empty() {
            return;
        }

        struct DeleteEdit {
            index: usize,
            start_char_idx: usize,
            len: usize,
            new_cursor: Cursor,
        }

        let mut edits = Vec::with_capacity(self.selections.len());

        for (index, selection) in self.selections.iter().enumerate() {
            if selection.is_collapsed() {
                let cursor = selection.active;
                let current_line_length = self.text_model.get_line(cursor.line).await
                    .map(|line| line.chars().count())
                    .unwrap_or(0);
                let line_offset = self.text_model.line_to_char(cursor.line).await;
                let char_idx = line_offset + cursor.column;
                
                if cursor.column < current_line_length {
                    edits.push(DeleteEdit {
                        index,
                        start_char_idx: char_idx,
                        len: 1,
                        new_cursor: cursor,
                    });
                } else {
                    let total_lines = self.text_model.line_count().await;
                    if cursor.line + 1 < total_lines {
                        edits.push(DeleteEdit {
                            index,
                            start_char_idx: char_idx,
                            len: 1,
                            new_cursor: cursor,
                        });
                    }
                }
            } else {
                let start = selection.start();
                let end = selection.end();
                let start_char_idx = self.text_model.line_to_char(start.line).await + start.column;
                let end_char_idx = self.text_model.line_to_char(end.line).await + end.column;
                if end_char_idx > start_char_idx {
                    edits.push(DeleteEdit {
                        index,
                        start_char_idx,
                        len: end_char_idx - start_char_idx,
                        new_cursor: start,
                    });
                }
            }
        }

        if edits.is_empty() {
            return;
        }

        edits.sort_by(|a, b| b.start_char_idx.cmp(&a.start_char_idx));
        for edit in &edits {
            self.text_model.remove(edit.start_char_idx, edit.len).await;
        }
        self.is_dirty = true;

        for edit in edits {
            if let Some(cursor_slot) = self.cursors.get_mut(edit.index) {
                *cursor_slot = edit.new_cursor;
            }
            if let Some(selection_slot) = self.selections.get_mut(edit.index) {
                *selection_slot = Selection::single(edit.new_cursor);
            }
        }
    }

    fn update_cursors_after_insert(&mut self, affected_indices: &[usize], inserted_text: &str) {
        let mut inserted_len = 0;
        let mut newline_count = 0;
        let mut last_line_len = 0;

        for ch in inserted_text.chars() {
            inserted_len += 1;
            if ch == '\n' {
                newline_count += 1;
                last_line_len = 0;
            } else {
                last_line_len += 1;
            }
        }

        let mut updates = Vec::with_capacity(affected_indices.len());

        for &index in affected_indices {
            if index >= self.selections.len() || index >= self.cursors.len() {
                continue;
            }

            let selection = self.selections[index];
            let mut new_cursor = if selection.is_collapsed() {
                selection.active
            } else {
                selection.start()
            };

            if newline_count == 0 {
                new_cursor.column += inserted_len;
            } else {
                new_cursor.line += newline_count;
                new_cursor.column = last_line_len;
            }

            updates.push((index, new_cursor));
        }

        for (index, new_cursor) in updates {
            if let Some(cursor_slot) = self.cursors.get_mut(index) {
                *cursor_slot = new_cursor;
            }
            if let Some(selection_slot) = self.selections.get_mut(index) {
                *selection_slot = Selection::single(new_cursor);
            }
        }
    }

    pub fn get_cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    pub fn get_selections(&self) -> &[Selection] {
        &self.selections
    }

    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.cursors = vec![cursor];
        self.selections = vec![Selection::single(cursor)];
    }

    pub fn add_cursor(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
        self.selections.push(Selection::single(cursor));
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.selections = vec![selection];
        self.cursors = vec![selection.active];
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    pub async fn line_count(&self) -> usize {
        self.text_model.line_count().await
    }

    pub async fn get_line(&self, line_idx: usize) -> Option<String> {
        self.text_model.get_line(line_idx).await
    }

    pub async fn get_line_length(&self, line_idx: usize) -> Option<usize> {
        self.text_model.get_line(line_idx).await.map(|line| line.chars().count())
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}
