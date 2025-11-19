use ropey::Rope;

pub trait RopeExt {
    fn to_string(&self) -> String;
    fn get_line_length(&self, line_idx: usize) -> Option<usize>;
    fn get_line_content(&self, line_idx: usize) -> Option<String>;
}

impl RopeExt for Rope {
    fn to_string(&self) -> String {
        let mut result = String::with_capacity(self.len_bytes());
        for chunk in self.chunks() {
            result.push_str(chunk);
        }
        result
    }

    fn get_line_length(&self, line_idx: usize) -> Option<usize> {
        if line_idx < self.len_lines() {
            let line = self.line(line_idx);
            Some(line.len_chars())
        } else {
            None
        }
    }

    fn get_line_content(&self, line_idx: usize) -> Option<String> {
        if line_idx < self.len_lines() {
            let line = self.line(line_idx);
            Some(line.to_string())
        } else {
            None
        }
    }
}