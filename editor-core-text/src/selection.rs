use super::cursor::Cursor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub anchor: Cursor,
    pub active: Cursor,
}

impl Selection {
    pub fn new(anchor: Cursor, active: Cursor) -> Self {
        Self { anchor, active }
    }

    pub fn single(cursor: Cursor) -> Self {
        Self {
            anchor: cursor,
            active: cursor,
        }
    }

    pub fn range(start: Cursor, end: Cursor) -> Self {
        Self {
            anchor: start,
            active: end,
        }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.active
    }

    pub fn start(&self) -> Cursor {
        if self.anchor.line < self.active.line
            || (self.anchor.line == self.active.line && self.anchor.column <= self.active.column)
        {
            self.anchor
        } else {
            self.active
        }
    }

    pub fn end(&self) -> Cursor {
        if self.anchor.line > self.active.line
            || (self.anchor.line == self.active.line && self.anchor.column > self.active.column)
        {
            self.anchor
        } else {
            self.active
        }
    }

    pub fn contains(&self, cursor: Cursor) -> bool {
        let start = self.start();
        let end = self.end();

        (cursor.line > start.line || (cursor.line == start.line && cursor.column >= start.column))
            && (cursor.line < end.line || (cursor.line == end.line && cursor.column <= end.column))
    }

    pub fn expand_to_line(&self) -> Self {
        let start = Cursor::new(self.start().line, 0);
        let end = Cursor::new(self.end().line, usize::MAX); // Will be clamped to actual line length
        Self::new(start, end)
    }
}
