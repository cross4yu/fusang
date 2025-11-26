use super::{cursor::Cursor, selection::Selection, text_model::TextModel};
use std::mem::size_of;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Buffer {
    text_model: Arc<TextModel>,
    cursors: Vec<Cursor>,
    selections: Vec<Selection>,
    is_dirty: bool,
    undo_stack: Vec<UndoRecord>,
    redo_stack: Vec<UndoRecord>,
    undo_stack_cost: usize,
}

#[derive(Debug, Clone)]
struct DeleteEdit {
    index: usize,
    start_char_idx: usize,
    len: usize,
    new_cursor: Cursor,
    deleted_text: String,
}

#[derive(Debug, Clone, Copy)]
enum DeleteDirection {
    Backward,
    Forward,
}

#[derive(Debug, Clone)]
struct ReplaceEdit {
    start_char_idx: usize,
    replaced_text: String,
}

#[derive(Debug, Clone)]
enum UndoRecord {
    Insert {
        edits: Vec<ReplaceEdit>,
        inserted_texts: Vec<String>,
        before_cursors: Vec<Cursor>,
        before_selections: Vec<Selection>,
        after_cursors: Vec<Cursor>,
        after_selections: Vec<Selection>,
        timestamp: Instant,
    },
    Delete {
        edits: Vec<DeleteEdit>,
        before_cursors: Vec<Cursor>,
        before_selections: Vec<Selection>,
        after_cursors: Vec<Cursor>,
        after_selections: Vec<Selection>,
        timestamp: Instant,
    },
}

impl UndoRecord {
    fn timestamp(&self) -> Instant {
        match self {
            UndoRecord::Insert { timestamp, .. } => *timestamp,
            UndoRecord::Delete { timestamp, .. } => *timestamp,
        }
    }

    fn cost(&self) -> usize {
        match self {
            UndoRecord::Insert {
                edits,
                inserted_texts,
                before_cursors,
                before_selections,
                after_cursors,
                after_selections,
                ..
            } => {
                edits
                    .iter()
                    .map(|edit| edit.replaced_text.len())
                    .sum::<usize>()
                    + inserted_texts.iter().map(|t| t.len()).sum::<usize>()
                    + before_cursors.len() * size_of::<Cursor>()
                    + before_selections.len() * size_of::<Selection>()
                    + after_cursors.len() * size_of::<Cursor>()
                    + after_selections.len() * size_of::<Selection>()
            }
            UndoRecord::Delete {
                edits,
                before_cursors,
                before_selections,
                after_cursors,
                after_selections,
                ..
            } => {
                edits
                    .iter()
                    .map(|edit| edit.deleted_text.len())
                    .sum::<usize>()
                    + before_cursors.len() * size_of::<Cursor>()
                    + before_selections.len() * size_of::<Selection>()
                    + after_cursors.len() * size_of::<Cursor>()
                    + after_selections.len() * size_of::<Selection>()
            }
        }
    }

    fn try_merge(&mut self, other: &UndoRecord) -> bool {
        match self {
            UndoRecord::Insert {
                edits: edits_a,
                inserted_texts: texts_a,
                after_cursors: after_cursors_a,
                after_selections: after_selections_a,
                ..
            } => {
                if let UndoRecord::Insert {
                    edits: edits_b,
                    inserted_texts: texts_b,
                    after_cursors,
                    after_selections,
                    ..
                } = other
                {
                    if edits_a.len() != edits_b.len()
                        || texts_a.len() != edits_a.len()
                        || texts_b.len() != edits_b.len()
                    {
                        return false;
                    }

                    for i in 0..edits_a.len() {
                        let edit_a = &edits_a[i];
                        let edit_b = &edits_b[i];
                        let text_a = &texts_a[i];
                        if !edit_a.replaced_text.is_empty() || !edit_b.replaced_text.is_empty() {
                            return false;
                        }
                        let len_a = text_a.chars().count();
                        if edit_b.start_char_idx != edit_a.start_char_idx + len_a {
                            return false;
                        }
                    }

                    for i in 0..texts_a.len() {
                        texts_a[i].push_str(&texts_b[i]);
                    }
                    *after_cursors_a = after_cursors.clone();
                    *after_selections_a = after_selections.clone();
                    true
                } else {
                    false
                }
            }
            UndoRecord::Delete {
                edits: edits_a,
                after_cursors: after_cursors_a,
                after_selections: after_selections_a,
                ..
            } => {
                if let UndoRecord::Delete {
                    edits: edits_b,
                    after_cursors,
                    after_selections,
                    ..
                } = other
                {
                    if edits_a.len() != edits_b.len() {
                        return false;
                    }

                    #[derive(Clone, Copy, PartialEq, Eq)]
                    enum MergeDir {
                        Backward,
                        Forward,
                    }

                    let mut direction: Option<MergeDir> = None;

                    for (edit_a, edit_b) in edits_a.iter_mut().zip(edits_b.iter()) {
                        if edit_a.index != edit_b.index {
                            return false;
                        }

                        let start_a = edit_a.start_char_idx;
                        let end_a = edit_a.start_char_idx + edit_a.len;
                        let start_b = edit_b.start_char_idx;
                        let end_b = edit_b.start_char_idx + edit_b.len;

                        let current_dir = if end_b == start_a {
                            MergeDir::Backward
                        } else if end_a == start_b {
                            MergeDir::Forward
                        } else {
                            return false;
                        };

                        match direction {
                            Some(dir) if dir != current_dir => return false,
                            None => direction = Some(current_dir),
                            _ => {}
                        }

                        match current_dir {
                            MergeDir::Backward => {
                                edit_a.start_char_idx = start_b;
                                edit_a.len += edit_b.len;
                                edit_a.deleted_text =
                                    format!("{}{}", edit_b.deleted_text, edit_a.deleted_text);
                                edit_a.new_cursor = edit_b.new_cursor;
                            }
                            MergeDir::Forward => {
                                edit_a.len += edit_b.len;
                                edit_a.deleted_text.push_str(&edit_b.deleted_text);
                                edit_a.new_cursor = edit_b.new_cursor;
                            }
                        }
                    }

                    *after_cursors_a = after_cursors.clone();
                    *after_selections_a = after_selections.clone();
                    true
                } else {
                    false
                }
            } // _ => false,
        }
    }
}

const COALESCE_WINDOW: Duration = Duration::from_millis(750);
const UNDO_STACK_BUDGET_BYTES: usize = 5 * 1024 * 1024; // ~5MB

impl Buffer {
    pub fn new() -> Self {
        Self {
            text_model: Arc::new(TextModel::new()),
            cursors: vec![Cursor::zero()],
            selections: vec![Selection::single(Cursor::zero())],
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            undo_stack_cost: 0,
        }
    }

    pub fn from_text(text: &str) -> Self {
        Self {
            text_model: Arc::new(TextModel::from_str(text)),
            cursors: vec![Cursor::zero()],
            selections: vec![Selection::single(Cursor::zero())],
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            undo_stack_cost: 0,
        }
    }

    pub async fn get_text(&self) -> String {
        self.text_model.get_text().await
    }

    pub async fn insert_text_at_cursor(&mut self, text: &str) {
        if self.selections.is_empty() {
            return;
        }
        if text.is_empty() {
            return;
        }

        struct SelectionEdit {
            index: usize,
            start_char_idx: usize,
            end_char_idx: usize,
            collapsed: bool,
            replaced_text: String,
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

            let replaced_text = if end_char_idx > start_char_idx {
                self.text_model
                    .get_text_range(start_char_idx, end_char_idx)
                    .await
            } else {
                String::new()
            };

            edits.push(SelectionEdit {
                index,
                start_char_idx,
                end_char_idx,
                collapsed,
                replaced_text,
            });
        }

        let touched_indices: Vec<usize> = edits.iter().map(|edit| edit.index).collect();

        // Normalize overlapping selections by merging their ranges.
        edits.sort_by(|a, b| a.start_char_idx.cmp(&b.start_char_idx));
        let mut normalized: Vec<SelectionEdit> = Vec::with_capacity(edits.len());
        for edit in edits {
            if let Some(last) = normalized.last_mut() {
                if edit.start_char_idx < last.end_char_idx {
                    last.end_char_idx = last.end_char_idx.max(edit.end_char_idx);
                    last.collapsed = false;
                    last.replaced_text = self
                        .text_model
                        .get_text_range(last.start_char_idx, last.end_char_idx)
                        .await;
                    continue;
                }
            }
            normalized.push(edit);
        }

        if normalized.is_empty() {
            return;
        }

        let before_cursors = self.cursors.clone();
        let before_selections = self.selections.clone();
        let inserted_texts = normalized
            .iter()
            .map(|_| text.to_string())
            .collect::<Vec<_>>();

        // Apply edits from the end of the buffer to avoid adjusting subsequent char indices
        normalized.sort_by(|a, b| b.start_char_idx.cmp(&a.start_char_idx));
        for edit in &normalized {
            if edit.collapsed {
                self.text_model.insert(edit.start_char_idx, text).await;
            } else {
                let length = edit.end_char_idx.saturating_sub(edit.start_char_idx);
                self.text_model
                    .replace(edit.start_char_idx, length, text)
                    .await;
            }
        }

        self.is_dirty = true;
        self.update_cursors_after_insert(&touched_indices, text);

        let after_cursors = self.cursors.clone();
        let after_selections = self.selections.clone();
        let replace_edits = normalized
            .iter()
            .map(|edit| ReplaceEdit {
                start_char_idx: edit.start_char_idx,
                replaced_text: edit.replaced_text.clone(),
            })
            .collect();

        let timestamp = Instant::now();
        self.record_operation(UndoRecord::Insert {
            edits: replace_edits,
            inserted_texts,
            before_cursors,
            before_selections,
            after_cursors,
            after_selections,
            timestamp,
        });
    }

    pub async fn insert_text_at_position(&mut self, line: usize, column: usize, text: &str) {
        if text.is_empty() {
            return;
        }

        self.set_cursor(Cursor::new(line, column));
        self.insert_text_at_cursor(text).await;
    }

    pub async fn insert_line_break(&mut self) {
        self.insert_text_at_cursor("\n").await;
    }

    pub async fn insert_tab(&mut self, tab_size: usize) {
        let spaces = " ".repeat(tab_size);
        self.insert_text_at_cursor(&spaces).await;
    }

    pub async fn delete_backward(&mut self) {
        let edits = self.collect_delete_edits(DeleteDirection::Backward).await;
        if edits.is_empty() {
            return;
        }
        let before_cursors = self.cursors.clone();
        let before_selections = self.selections.clone();
        self.apply_delete_edits(edits.clone()).await;
        let after_cursors = self.cursors.clone();
        let after_selections = self.selections.clone();
        let timestamp = Instant::now();
        self.record_operation(UndoRecord::Delete {
            edits,
            before_cursors,
            before_selections,
            after_cursors,
            after_selections,
            timestamp,
        });
    }

    pub async fn delete_forward(&mut self) {
        let edits = self.collect_delete_edits(DeleteDirection::Forward).await;
        if edits.is_empty() {
            return;
        }
        let before_cursors = self.cursors.clone();
        let before_selections = self.selections.clone();
        self.apply_delete_edits(edits.clone()).await;
        let after_cursors = self.cursors.clone();
        let after_selections = self.selections.clone();
        let timestamp = Instant::now();
        self.record_operation(UndoRecord::Delete {
            edits,
            before_cursors,
            before_selections,
            after_cursors,
            after_selections,
            timestamp,
        });
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

    async fn collect_delete_edits(&self, direction: DeleteDirection) -> Vec<DeleteEdit> {
        if self.selections.is_empty() {
            return Vec::new();
        }

        let mut edits = Vec::with_capacity(self.selections.len());

        for (index, selection) in self.selections.iter().enumerate() {
            if selection.is_collapsed() {
                if let Some(edit) = self
                    .build_collapsed_delete_edit(index, selection.active, direction)
                    .await
                {
                    edits.push(edit);
                }
            } else {
                let start = selection.start();
                let end = selection.end();
                let start_char_idx = self.text_model.line_to_char(start.line).await + start.column;
                let end_char_idx = self.text_model.line_to_char(end.line).await + end.column;
                if end_char_idx > start_char_idx {
                    let deleted_text = self
                        .text_model
                        .get_text_range(start_char_idx, end_char_idx)
                        .await;
                    edits.push(DeleteEdit {
                        index,
                        start_char_idx,
                        len: end_char_idx - start_char_idx,
                        new_cursor: start,
                        deleted_text,
                    });
                }
            }
        }

        edits
    }

    async fn build_collapsed_delete_edit(
        &self,
        index: usize,
        cursor: Cursor,
        direction: DeleteDirection,
    ) -> Option<DeleteEdit> {
        match direction {
            DeleteDirection::Backward => {
                if cursor.column > 0 {
                    let line_offset = self.text_model.line_to_char(cursor.line).await;
                    let char_idx = line_offset + cursor.column;
                    let deleted_text = self.text_model.get_text_range(char_idx - 1, char_idx).await;
                    Some(DeleteEdit {
                        index,
                        start_char_idx: char_idx - 1,
                        len: 1,
                        new_cursor: Cursor::new(cursor.line, cursor.column - 1),
                        deleted_text,
                    })
                } else if cursor.line > 0 {
                    let line_start = self.text_model.line_to_char(cursor.line).await;
                    if line_start == 0 {
                        return None;
                    }
                    let prev_line_length = self
                        .text_model
                        .get_line(cursor.line - 1)
                        .await
                        .map(|line| line.chars().count())
                        .unwrap_or(0);
                    let deleted_text = self
                        .text_model
                        .get_text_range(line_start - 1, line_start)
                        .await;
                    Some(DeleteEdit {
                        index,
                        start_char_idx: line_start - 1,
                        len: 1,
                        new_cursor: Cursor::new(cursor.line - 1, prev_line_length),
                        deleted_text,
                    })
                } else {
                    None
                }
            }
            DeleteDirection::Forward => {
                let current_line_length = self
                    .text_model
                    .get_line(cursor.line)
                    .await
                    .map(|line| line.chars().count())
                    .unwrap_or(0);
                let line_offset = self.text_model.line_to_char(cursor.line).await;
                let char_idx = line_offset + cursor.column;

                if cursor.column < current_line_length {
                    let deleted_text = self.text_model.get_text_range(char_idx, char_idx + 1).await;
                    Some(DeleteEdit {
                        index,
                        start_char_idx: char_idx,
                        len: 1,
                        new_cursor: cursor,
                        deleted_text,
                    })
                } else {
                    let total_lines = self.text_model.line_count().await;
                    if cursor.line + 1 < total_lines {
                        let deleted_text =
                            self.text_model.get_text_range(char_idx, char_idx + 1).await;
                        Some(DeleteEdit {
                            index,
                            start_char_idx: char_idx,
                            len: 1,
                            new_cursor: cursor,
                            deleted_text,
                        })
                    } else {
                        None
                    }
                }
            }
        }
    }

    async fn apply_delete_edits(&mut self, mut edits: Vec<DeleteEdit>) {
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

    pub async fn cursor_char_index(&self, cursor: Cursor) -> usize {
        self.text_model.line_to_char(cursor.line).await + cursor.column
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
        self.text_model
            .get_line(line_idx)
            .await
            .map(|line| line.chars().count())
    }

    pub async fn undo(&mut self) -> bool {
        if let Some(record) = self.undo_stack.pop() {
            self.undo_stack_cost = self.undo_stack_cost.saturating_sub(record.cost());
            self.apply_undo(&record).await;
            self.redo_stack.push(record);
            if self.undo_stack.is_empty() {
                self.is_dirty = false;
            }
            if self.undo_stack.is_empty() {
                self.is_dirty = false;
            }
            true
        } else {
            false
        }
    }

    pub async fn redo(&mut self) -> bool {
        if let Some(record) = self.redo_stack.pop() {
            self.apply_redo(&record).await;
            self.push_undo_record_inner(record);
            true
        } else {
            false
        }
    }

    fn record_operation(&mut self, operation: UndoRecord) {
        let mut merged = false;
        if let Some(last) = self.undo_stack.last_mut() {
            if let Some(delta) = operation
                .timestamp()
                .checked_duration_since(last.timestamp())
            {
                if delta <= COALESCE_WINDOW {
                    let prev_cost = last.cost();
                    if last.try_merge(&operation) {
                        let new_cost = last.cost();
                        self.undo_stack_cost = self
                            .undo_stack_cost
                            .saturating_sub(prev_cost)
                            .saturating_add(new_cost);
                        merged = true;
                    }
                }
            }
        }
        if merged {
            self.trim_undo_stack();
        } else {
            self.push_undo_record_inner(operation);
        }
        self.redo_stack.clear();
    }

    fn push_undo_record_inner(&mut self, record: UndoRecord) {
        self.undo_stack_cost = self.undo_stack_cost.saturating_add(record.cost());
        self.undo_stack.push(record);
        self.trim_undo_stack();
    }

    fn trim_undo_stack(&mut self) {
        while self.undo_stack_cost > UNDO_STACK_BUDGET_BYTES && !self.undo_stack.is_empty() {
            let removed = self.undo_stack.remove(0);
            self.undo_stack_cost = self.undo_stack_cost.saturating_sub(removed.cost());
        }
        if self.undo_stack.is_empty() {
            self.is_dirty = false;
        }
    }

    async fn apply_undo(&mut self, record: &UndoRecord) {
        match record {
            UndoRecord::Insert {
                edits,
                inserted_texts,
                before_cursors,
                before_selections,
                ..
            } => {
                let mut ordered: Vec<(ReplaceEdit, String)> = edits
                    .iter()
                    .cloned()
                    .zip(inserted_texts.iter().cloned())
                    .collect();
                ordered.sort_by(|a, b| b.0.start_char_idx.cmp(&a.0.start_char_idx));
                for (edit, inserted) in ordered {
                    let inserted_len = inserted.chars().count();
                    if inserted_len > 0 {
                        self.text_model
                            .remove(edit.start_char_idx, inserted_len)
                            .await;
                    }
                    if !edit.replaced_text.is_empty() {
                        self.text_model
                            .insert(edit.start_char_idx, &edit.replaced_text)
                            .await;
                    }
                }
                self.cursors = before_cursors.clone();
                self.selections = before_selections.clone();
                self.is_dirty = true;
            }
            UndoRecord::Delete {
                edits,
                before_cursors,
                before_selections,
                ..
            } => {
                let mut ordered = edits.clone();
                ordered.sort_by(|a, b| a.start_char_idx.cmp(&b.start_char_idx));
                for edit in ordered {
                    self.text_model
                        .insert(edit.start_char_idx, &edit.deleted_text)
                        .await;
                }
                self.cursors = before_cursors.clone();
                self.selections = before_selections.clone();
                self.is_dirty = true;
            }
        }
    }

    async fn apply_redo(&mut self, record: &UndoRecord) {
        match record {
            UndoRecord::Insert {
                edits,
                inserted_texts,
                after_cursors,
                after_selections,
                ..
            } => {
                let mut ordered: Vec<(ReplaceEdit, String)> = edits
                    .iter()
                    .cloned()
                    .zip(inserted_texts.iter().cloned())
                    .collect();
                ordered.sort_by(|a, b| b.0.start_char_idx.cmp(&a.0.start_char_idx));
                for (edit, inserted) in ordered {
                    if !edit.replaced_text.is_empty() {
                        let len = edit.replaced_text.chars().count();
                        self.text_model.remove(edit.start_char_idx, len).await;
                    }
                    self.text_model.insert(edit.start_char_idx, &inserted).await;
                }
                self.cursors = after_cursors.clone();
                self.selections = after_selections.clone();
                self.is_dirty = true;
            }
            UndoRecord::Delete {
                edits,
                after_cursors,
                after_selections,
                ..
            } => {
                self.apply_delete_edits(edits.clone()).await;
                self.cursors = after_cursors.clone();
                self.selections = after_selections.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use tokio::runtime::Runtime;

    fn run_async<T>(future: T) -> T::Output
    where
        T: Future,
    {
        Runtime::new()
            .expect("failed to create Tokio runtime for buffer tests")
            .block_on(future)
    }

    #[test]
    fn multi_line_insert_updates_cursor_position() {
        run_async(async {
            let mut buffer = Buffer::new();
            buffer.insert_text_at_cursor("foo\nbar").await;

            let text = buffer.get_text().await;
            assert_eq!(text, "foo\nbar");

            let cursor = buffer.get_cursors()[0];
            assert_eq!(cursor.line, 1);
            assert_eq!(cursor.column, 3);
        });
    }

    #[test]
    fn multi_cursor_insert_keeps_unaffected_cursors() {
        run_async(async {
            let mut buffer = Buffer::from_text("abcd");

            buffer.selections = vec![
                Selection::single(Cursor::new(0, 1)),
                Selection::single(Cursor::new(0, 3)),
            ];
            buffer.cursors = buffer
                .selections
                .iter()
                .map(|selection| selection.active)
                .collect();

            let untouched_cursor = Cursor::new(0, 0);
            buffer.cursors.push(untouched_cursor);

            buffer.insert_text_at_cursor("X").await;

            let text = buffer.get_text().await;
            assert_eq!(text, "aXbcXd");

            assert_eq!(buffer.cursors[0], Cursor::new(0, 2));
            assert_eq!(buffer.cursors[1], Cursor::new(0, 4));
            assert_eq!(buffer.cursors[2], untouched_cursor);
        });
    }

    #[test]
    fn multi_cursor_delete_respects_each_selection() {
        run_async(async {
            let mut buffer = Buffer::from_text("abcdef");

            buffer.selections = vec![
                Selection::single(Cursor::new(0, 2)),
                Selection::range(Cursor::new(0, 3), Cursor::new(0, 5)),
            ];
            buffer.cursors = buffer
                .selections
                .iter()
                .map(|selection| selection.active)
                .collect();

            buffer.delete_backward().await;

            let text = buffer.get_text().await;
            assert_eq!(text, "abf");

            assert_eq!(buffer.cursors[0], Cursor::new(0, 1));
            assert_eq!(buffer.cursors[1], Cursor::new(0, 2));
        });
    }

    #[test]
    fn multi_cursor_forward_delete_removes_expected_characters() {
        run_async(async {
            let mut buffer = Buffer::from_text("abcdef");

            buffer.selections = vec![
                Selection::single(Cursor::new(0, 1)),
                Selection::single(Cursor::new(0, 3)),
            ];
            buffer.cursors = buffer
                .selections
                .iter()
                .map(|selection| selection.active)
                .collect();

            buffer.delete_forward().await;

            let text = buffer.get_text().await;
            assert_eq!(text, "acef");

            assert_eq!(buffer.cursors[0], Cursor::new(0, 1));
            assert_eq!(buffer.cursors[1], Cursor::new(0, 3));
        });
    }

    #[test]
    fn overlapping_selections_are_merged_before_insert() {
        run_async(async {
            let mut buffer = Buffer::from_text("abcdef");

            buffer.selections = vec![
                Selection::range(Cursor::new(0, 1), Cursor::new(0, 4)),
                Selection::range(Cursor::new(0, 2), Cursor::new(0, 5)),
            ];
            buffer.cursors = buffer
                .selections
                .iter()
                .map(|selection| selection.active)
                .collect();

            buffer.insert_text_at_cursor("X").await;

            let text = buffer.get_text().await;
            assert_eq!(text, "aXf");

            let cursor = buffer.get_cursors()[0];
            assert_eq!(cursor.line, 0);
            assert_eq!(cursor.column, 2);
        });
    }

    #[test]
    fn undo_and_redo_restore_text_and_cursors() {
        run_async(async {
            let mut buffer = Buffer::from_text("abc");
            buffer.insert_text_at_cursor("X").await;
            assert_eq!(buffer.get_text().await, "Xabc");
            assert_eq!(buffer.get_cursors()[0], Cursor::new(0, 1));

            assert!(buffer.undo().await);
            assert_eq!(buffer.get_text().await, "abc");
            assert_eq!(buffer.get_cursors()[0], Cursor::new(0, 0));

            assert!(buffer.redo().await);
            assert_eq!(buffer.get_text().await, "Xabc");
            assert_eq!(buffer.get_cursors()[0], Cursor::new(0, 1));
        });
    }

    #[test]
    fn redo_stack_cleared_after_new_edit() {
        run_async(async {
            let mut buffer = Buffer::from_text("abc");
            buffer.insert_text_at_cursor("X").await;
            assert!(buffer.undo().await);

            buffer.insert_text_at_cursor("Y").await;
            assert_eq!(buffer.get_text().await, "Yabc");
            assert!(!buffer.redo().await);
        });
    }

    #[test]
    fn sequential_typing_coalesces_into_single_undo() {
        run_async(async {
            let mut buffer = Buffer::new();
            buffer.insert_text_at_cursor("a").await;
            buffer.insert_text_at_cursor("b").await;
            buffer.insert_text_at_cursor("c").await;

            assert_eq!(buffer.get_text().await, "abc");
            assert!(buffer.undo().await);
            assert_eq!(buffer.get_text().await, "");
            assert!(!buffer.undo().await);
            assert!(buffer.redo().await);
            assert_eq!(buffer.get_text().await, "abc");
        });
    }

    #[test]
    fn multi_cursor_typing_coalesces_into_single_undo() {
        run_async(async {
            let mut buffer = Buffer::from_text("wxyz");
            buffer.selections = vec![
                Selection::single(Cursor::new(0, 1)),
                Selection::single(Cursor::new(0, 3)),
            ];
            buffer.cursors = buffer.selections.iter().map(|sel| sel.active).collect();

            buffer.insert_text_at_cursor("a").await;
            buffer.insert_text_at_cursor("b").await;

            let modified = buffer.get_text().await;
            assert_ne!(modified, "wxyz");
            assert!(buffer.undo().await);
            assert_eq!(buffer.get_text().await, "wxyz");
            assert!(buffer.redo().await);
            assert_eq!(buffer.get_text().await, modified);
        });
    }

    #[test]
    fn sequential_backspaces_coalesce() {
        run_async(async {
            let mut buffer = Buffer::from_text("abc");
            buffer.delete_backward().await;
            buffer.delete_backward().await;
            buffer.delete_backward().await;

            assert_eq!(buffer.get_text().await, "");
            assert!(buffer.undo().await);
            assert_eq!(buffer.get_text().await, "abc");
        });
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}
