use std::collections::HashMap;
use std::io;

use super::buffer::Buffer;

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

    pub fn save_all(&mut self) -> io::Result<()> {
        for buffer in self.items.values_mut() {
            if buffer.is_dirty() {
                buffer.save_to_disk()?;
            }
        }

        Ok(())
    }

    pub fn save(&mut self, name: &str) -> io::Result<()> {
        if let Some(buffer) = self.items.get_mut(name) {
            buffer.save_to_disk()
        } else {
            Ok(())
        }
    }

    pub fn save_if_dirty(&mut self, name: &str) -> io::Result<bool> {
        if let Some(buffer) = self.items.get_mut(name) {
            if buffer.is_dirty() {
                buffer.save_to_disk()?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn is_dirty(&self, name: &str) -> bool {
        self.items
            .get(name)
            .map(|buffer| buffer.is_dirty())
            .unwrap_or(false)
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
