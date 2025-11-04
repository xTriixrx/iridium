use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct BufferStore {
    items: HashMap<String, Buffer>,
}

impl BufferStore {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    pub fn open(&mut self, name: impl Into<String>) -> &mut Buffer {
        let key = name.into();
        self.items
            .entry(key.clone())
            .or_insert_with(|| Buffer::new(key))
    }

    pub fn get(&self, name: &str) -> Option<&Buffer> {
        self.items.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Buffer> {
        self.items.get_mut(name)
    }

    pub fn list(&self) -> Vec<String> {
        self.items.keys().cloned().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn insert_char(&mut self, name: &str, row: usize, col: usize, ch: char) {
        let buffer = self
            .items
            .entry(name.to_string())
            .or_insert_with(|| Buffer::new(name.to_string()));
        buffer.insert_char(row, col, ch);
    }

    pub fn save_all(&self) -> io::Result<()> {
        for buffer in self.items.values() {
            buffer.save_to_disk()?;
        }

        Ok(())
    }

    pub fn delete_char(&mut self, name: &str, row: usize, col: usize) -> Option<(usize, usize)> {
        let buffer = self.items.get_mut(name)?;
        buffer.delete_char(row, col)
    }

    pub fn insert_newline(&mut self, name: &str, row: usize, col: usize) -> (usize, usize) {
        let buffer = self
            .items
            .entry(name.to_string())
            .or_insert_with(|| Buffer::new(name.to_string()));
        buffer.insert_newline(row, col)
    }

    pub fn pad_line(&mut self, name: &str, row: usize, width: usize) {
        let buffer = self
            .items
            .entry(name.to_string())
            .or_insert_with(|| Buffer::new(name.to_string()));
        buffer.pad_line(row, width);
    }
}

#[derive(Debug, Clone, Default)]
pub struct Buffer {
    name: String,
    lines: Vec<String>,
}

impl Buffer {
    fn new(name: String) -> Self {
        Self {
            name,
            lines: Vec::new(),
        }
    }

    pub fn append(&mut self, line: String) {
        self.lines.push(line);
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn remove_last(&mut self) -> Option<String> {
        self.lines.pop()
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
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    fn save_to_disk(&self) -> io::Result<()> {
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

        Ok(())
    }

    fn delete_char(&mut self, row: usize, col: usize) -> Option<(usize, usize)> {
        let line = self.lines.get_mut(row)?;
        let char_count = line.chars().count();
        if col == 0 || col > char_count {
            return None;
        }

        let start = Self::byte_index(line, col - 1);
        let end = Self::byte_index(line, col);
        line.replace_range(start..end, "");
        Some((row, col - 1))
    }

    fn insert_newline(&mut self, row: usize, col: usize) -> (usize, usize) {
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
        (row + 1, 0)
    }

    fn pad_line(&mut self, row: usize, width: usize) {
        while self.lines.len() <= row {
            self.lines.push(String::new());
        }

        if let Some(line) = self.lines.get_mut(row) {
            let char_count = line.chars().count();
            if char_count < width {
                line.push_str(&" ".repeat(width - char_count));
            }
        }
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
