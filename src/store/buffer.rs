use crate::store::buffer_snapshot::BufferSnapshot;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Represents the editable contents of a named buffer in memory.
///
/// `Buffer` tracks the in-memory lines, dirty state, and persistence helpers
/// that back the editor UI and shell commands.
#[derive(Debug, Clone, Default)]
pub struct Buffer {
    name: String,
    lines: Vec<String>,
    dirty: bool,
    requires_name: bool,
    is_open: bool,
}

impl Buffer {
    /// Create a new, empty buffer associated with the provided path or name.
    pub(crate) fn new(name: String) -> Self {
        Self::with_name_state(name, false)
    }

    pub(crate) fn new_untitled(name: String) -> Self {
        Self::with_name_state(name, true)
    }

    fn with_name_state(name: String, requires_name: bool) -> Self {
        Self {
            name,
            lines: Vec::new(),
            dirty: false,
            requires_name,
            is_open: true,
        }
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn set_open(&mut self, open: bool) {
        self.is_open = open;
    }

    /// Append a new line of text and mark the buffer dirty.
    pub fn append(&mut self, line: String) {
        self.lines.push(line);
        self.dirty = true;
    }

    /// Remove all lines from the buffer.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.dirty = true;
    }

    /// Remove the last line, returning it when present, and mark dirty.
    pub fn remove_last(&mut self) -> Option<String> {
        let popped = self.lines.pop();
        if popped.is_some() {
            self.dirty = true;
        }
        popped
    }

    /// Print the buffer contents or a placeholder if empty.
    pub fn print(&self) {
        if self.lines.is_empty() {
            println!("(buffer '{}' is empty)", self.name);
        } else {
            for line in &self.lines {
                println!("{line}");
            }
        }
    }

    /// Insert a character at a given row/column, padding as required.
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

    /// Immutable view of the underlying lines.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Persist the buffer contents to disk, clearing the dirty flag.
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

    /// Delete the character before the provided column, returning new cursor coordinates.
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

    /// Insert a newline at the provided location and return the cursor position after insertion.
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

    /// Ensure `row` exists and pad the line with spaces until it reaches `width`.
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

    /// Whether the buffer contains unsaved changes.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub(crate) fn set_name(&mut self, name: String) {
        self.name = name;
        self.requires_name = false;
    }

    pub(crate) fn requires_name(&self) -> bool {
        self.requires_name
    }

    pub(crate) fn mark_requires_name(&mut self, requires_name: bool) {
        self.requires_name = requires_name;
    }

    pub(crate) fn to_snapshot(&self) -> BufferSnapshot {
        BufferSnapshot::new(
            self.name.clone(),
            self.lines.clone(),
            self.requires_name,
            self.is_open,
            self.dirty,
        )
    }

    pub(crate) fn from_snapshot(snapshot: BufferSnapshot) -> Self {
        Self {
            name: snapshot.name,
            lines: snapshot.lines,
            dirty: snapshot.dirty,
            requires_name: snapshot.requires_name,
            is_open: snapshot.is_open,
        }
    }

    /// Translate a character index into a byte offset for utf-8 strings.
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

    /// Appending lines marks the buffer dirty while `clear` resets state.
    #[test]
    fn append_adds_lines_and_clear_resets() {
        let mut buffer = Buffer::new("test".into());
        buffer.append("first".into());
        buffer.append("second".into());

        assert_eq!(buffer.lines.len(), 2);
        buffer.clear();
        assert!(buffer.lines.is_empty());
    }

    /// Inserting and deleting characters updates both text and cursor metadata.
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

    /// Splitting a line and padding the continuation behaves as expected.
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

    /// Removing from an empty buffer is a no-op and leaves the dirty flag untouched.
    #[test]
    fn remove_last_on_empty_returns_none() {
        let mut buffer = Buffer::new("test".into());
        assert!(buffer.remove_last().is_none());
        assert!(!buffer.is_dirty());
    }

    /// Attempting to delete outside the valid range does nothing.
    #[test]
    fn delete_char_out_of_bounds_is_noop() {
        let mut buffer = Buffer::new("test".into());
        buffer.append("abc".into());
        buffer.dirty = false;

        assert!(buffer.delete_char(0, 0).is_none());
        assert!(!buffer.is_dirty());
    }

    /// Padding with a width shorter than the current line keeps the buffer clean.
    #[test]
    fn pad_line_with_shorter_width_keeps_clean_state() {
        let mut buffer = Buffer::new("test".into());
        buffer.append("abcd".into());
        buffer.dirty = false;

        buffer.pad_line(0, 2);
        assert_eq!(buffer.lines[0], "abcd");
        assert!(!buffer.is_dirty());
    }

    /// Removing existing lines returns them and marks the buffer dirty.
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

    /// Saving the buffer writes to disk and clears the dirty flag.
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
