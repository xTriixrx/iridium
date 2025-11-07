use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct Buffer {
    name: String,
    lines: Vec<String>,
    dirty: bool,
}

impl Buffer {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            lines: Vec::new(),
            dirty: false,
        }
    }
    pub fn append(&mut self, line: String) {
        self.lines.push(line);
        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.dirty = true;
    }

    pub fn remove_last(&mut self) -> Option<String> {
        let popped = self.lines.pop();
        if popped.is_some() {
            self.dirty = true;
        }
        popped
    }

    pub fn print(&self) {
        if self.lines.is_empty() {
            println!("(buffer '{}' is empty)", self.name);
        } else {
            for line in &self.lines {
                println!("{line}");
            }
        }
    }

    pub fn insert_char(&mut self, row: usize, col: usize, ch: char) {
        while self.lines.len() <= row {
            self.lines.push(String::new());
        }

        if let Some(line) = self.lines.get_mut(row) {
            let char_count = line.chars().count();
            if col > char_count {
                line.push_str(&" ".repeat(col - char_count));
            }

            if col >= char_count {
                line.push(ch);
            } else {
                let start = Self::byte_index(line, col);
                let end = Self::byte_index(line, col + 1);
                line.replace_range(start..end, &ch.to_string());
            }
            self.dirty = true;
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub(crate) fn save_to_disk(&mut self) -> io::Result<()> {
        let path = Path::new(&self.name);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let mut file = File::create(path)?;
        for line in &self.lines {
            writeln!(file, "{}", line)?;
        }

        self.dirty = false;
        Ok(())
    }

    pub(crate) fn delete_char(&mut self, row: usize, col: usize) -> Option<(usize, usize)> {
        let line = self.lines.get_mut(row)?;
        let char_count = line.chars().count();
        if col == 0 || col > char_count {
            return None;
        }

        let start = Self::byte_index(line, col - 1);
        let end = Self::byte_index(line, col);
        line.replace_range(start..end, "");
        self.dirty = true;
        Some((row, col - 1))
    }

    pub(crate) fn insert_newline(&mut self, row: usize, col: usize) -> (usize, usize) {
        while self.lines.len() <= row {
            self.lines.push(String::new());
        }

        let trailing = if let Some(line) = self.lines.get_mut(row) {
            let char_count = line.chars().count();
            if col > char_count {
                line.push_str(&" ".repeat(col - char_count));
            }
            let idx = Self::byte_index(line, col);
            line.split_off(idx)
        } else {
            String::new()
        };

        self.lines.insert(row + 1, trailing);
        self.dirty = true;
        (row + 1, 0)
    }

    pub(crate) fn pad_line(&mut self, row: usize, width: usize) {
        while self.lines.len() <= row {
            self.lines.push(String::new());
        }

        if let Some(line) = self.lines.get_mut(row) {
            let char_count = line.chars().count();
            if char_count < width {
                line.push_str(&" ".repeat(width - char_count));
                self.dirty = true;
            }
        }
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn byte_index(line: &str, char_idx: usize) -> usize {
        if char_idx == 0 {
            return 0;
        }

        let mut count = 0;
        for (idx, _) in line.char_indices() {
            if count == char_idx {
                return idx;
            }
            count += 1;
        }
        line.len()
    }
}

#[cfg(test)]
mod tests {
    use super::Buffer;
    use std::fs;
    use std::io::Read;

    #[test]
    fn append_adds_lines_and_clear_resets() {
        let mut buffer = Buffer::new("test".into());
        buffer.append("first".into());
        buffer.append("second".into());

        assert_eq!(buffer.lines.len(), 2);
        buffer.clear();
        assert!(buffer.lines.is_empty());
    }

    #[test]
    fn insert_and_delete_char_updates_content() {
        let mut buffer = Buffer::new("test".into());
        buffer.append("abc".into());

        buffer.insert_char(0, 1, 'X');
        assert_eq!(buffer.lines[0], "aXc");

        let pos = buffer.delete_char(0, 2).expect("delete should succeed");
        assert_eq!(pos, (0, 1));
        assert_eq!(buffer.lines[0], "ac");
    }

    #[test]
    fn insert_newline_and_pad_line_work() {
        let mut buffer = Buffer::new("test".into());
        buffer.append("hello".into());

        let (row, col) = buffer.insert_newline(0, 2);
        assert_eq!((row, col), (1, 0));
        assert_eq!(buffer.lines[0], "he");
        assert_eq!(buffer.lines[1], "llo");

        buffer.pad_line(1, 6);
        assert_eq!(buffer.lines[1], "llo   ");
    }

    #[test]
    fn remove_last_on_empty_returns_none() {
        let mut buffer = Buffer::new("test".into());
        assert!(buffer.remove_last().is_none());
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn delete_char_out_of_bounds_is_noop() {
        let mut buffer = Buffer::new("test".into());
        buffer.append("abc".into());
        buffer.dirty = false;

        assert!(buffer.delete_char(0, 0).is_none());
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn pad_line_with_shorter_width_keeps_clean_state() {
        let mut buffer = Buffer::new("test".into());
        buffer.append("abcd".into());
        buffer.dirty = false;

        buffer.pad_line(0, 2);
        assert_eq!(buffer.lines[0], "abcd");
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn remove_last_marks_dirty_and_returns_line() {
        let mut buffer = Buffer::new("test".into());
        buffer.append("alpha".into());
        buffer.append("beta".into());

        let removed = buffer.remove_last();
        assert_eq!(removed.as_deref(), Some("beta"));
        assert!(buffer.is_dirty());
        assert_eq!(buffer.lines(), &[String::from("alpha")]);
    }

    #[test]
    fn save_to_disk_persists_contents_and_clears_dirty_flag() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!(
            "iridium_buffer_unit_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path_str = path.to_string_lossy().to_string();

        let mut buffer = Buffer::new(path_str.clone());
        buffer.append("line 1".into());
        buffer.append("line 2".into());
        assert!(buffer.is_dirty());

        buffer.save_to_disk().expect("save_to_disk should succeed");
        assert!(!buffer.is_dirty());

        let mut file = fs::File::open(&path).expect("file should exist");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("should read file");
        assert_eq!(contents, "line 1\nline 2\n");

        let _ = fs::remove_file(&path);
    }
}
