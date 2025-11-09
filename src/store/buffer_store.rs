use std::collections::HashMap;
use std::io;

use super::buffer::Buffer;

/// In-memory manager that tracks named buffers and orchestrates their lifecycle.
///
/// `BufferStore` owns the canonical `Buffer` instances, provides lookup helpers,
/// coordinates persistence, and mediates text mutations for the editor and
/// control layers.
#[derive(Debug, Clone, Default)]
pub struct BufferStore {
    buffers: HashMap<String, Buffer>,
}

impl BufferStore {
    /// Construct an empty buffer store with no loaded buffers.
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// Fetch a mutable reference to the named buffer, creating it if necessary.
    pub fn open(&mut self, name: impl Into<String>) -> &mut Buffer {
        self.open_with_state(name, false)
    }

    /// Create an untitled buffer that still requires a user-supplied name.
    pub fn open_untitled(&mut self, name: impl Into<String>) -> &mut Buffer {
        self.open_with_state(name, true)
    }

    fn open_with_state(&mut self, name: impl Into<String>, requires_name: bool) -> &mut Buffer {
        let key = name.into();

        let buffer = self.buffers.entry(key.clone()).or_insert_with(|| {
            if requires_name {
                Buffer::new_untitled(key.clone())
            } else {
                Buffer::new(key.clone())
            }
        });
        buffer.set_open(true);
        buffer
    }

    /// Retrieve an immutable reference to a buffer when available.
    pub fn get(&self, name: &str) -> Option<&Buffer> {
        self.buffers.get(name)
    }

    /// Retrieve a mutable reference to a buffer when available.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Buffer> {
        self.buffers.get_mut(name)
    }

    /// Return a vector of the buffer names currently tracked in the active set.
    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = self.buffers.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn open_buffers(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .buffers
            .iter()
            .filter_map(|(name, buffer)| buffer.is_open().then(|| name.clone()))
            .collect();
        names.sort();
        names
    }

    /// Report whether the store contains any buffers.
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Insert a character at the requested coordinates, growing the buffer as needed.
    pub fn insert_char(&mut self, name: &str, row: usize, col: usize, ch: char) {
        let buffer = self
            .buffers
            .entry(name.to_string())
            .or_insert_with(|| Buffer::new(name.to_string()));
        buffer.insert_char(row, col, ch);
    }

    /// Save every dirty buffer to disk.
    pub fn save_all(&mut self) -> io::Result<()> {
        for buffer in self.buffers.values_mut() {
            if buffer.is_dirty() {
                buffer.save_to_disk()?;
            }
        }

        Ok(())
    }

    /// Save a specific buffer to disk when it exists.
    pub fn save(&mut self, name: &str) -> io::Result<()> {
        if let Some(buffer) = self.buffers.get_mut(name) {
            buffer.save_to_disk()
        } else {
            Ok(())
        }
    }

    /// Persist a buffer only if it is dirty, returning whether a write occurred.
    pub fn save_if_dirty(&mut self, name: &str) -> io::Result<bool> {
        if let Some(buffer) = self.buffers.get_mut(name) {
            if buffer.is_dirty() {
                buffer.save_to_disk()?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Mark a buffer clean without writing it to disk.
    pub fn save_in_memory(&mut self, name: &str) -> bool {
        if let Some(buffer) = self.buffers.get_mut(name) {
            buffer.mark_clean();
            return true;
        }
        false
    }

    /// Determine if the named buffer has unsaved changes.
    pub fn is_dirty(&self, name: &str) -> bool {
        self.buffers
            .get(name)
            .map(|buffer| buffer.is_dirty())
            .unwrap_or(false)
    }

    /// Whether the buffer still needs to be given a user-specified name.
    pub fn requires_name(&self, name: &str) -> bool {
        self.buffers
            .get(name)
            .map(|buffer| buffer.requires_name())
            .unwrap_or(false)
    }

    /// Delete a character preceding the provided column, returning the new cursor position.
    pub fn delete_char(&mut self, name: &str, row: usize, col: usize) -> Option<(usize, usize)> {
        let buffer = self.buffers.get_mut(name)?;
        buffer.delete_char(row, col)
    }

    /// Insert a newline at the specified location, splitting or padding as needed.
    pub fn insert_newline(&mut self, name: &str, row: usize, col: usize) -> (usize, usize) {
        let buffer = self
            .buffers
            .entry(name.to_string())
            .or_insert_with(|| Buffer::new(name.to_string()));
        buffer.insert_newline(row, col)
    }

    /// Pad the requested line with spaces so it reaches `width` characters.
    pub fn pad_line(&mut self, name: &str, row: usize, width: usize) {
        let buffer = self
            .buffers
            .entry(name.to_string())
            .or_insert_with(|| Buffer::new(name.to_string()));
        buffer.pad_line(row, width);
    }

    /// Mark a buffer as closed while leaving it in memory.
    pub fn mark_closed(&mut self, name: &str) -> bool {
        if let Some(buffer) = self.buffers.get_mut(name) {
            buffer.set_open(false);
            return true;
        }
        false
    }

    /// Remove the specified buffer from memory entirely, regardless of whether it was suspended.
    pub fn remove(&mut self, name: &str) -> bool {
        if self.buffers.remove(name).is_some() {
            return true;
        }
        return false;
        // self.closed.remove(name).is_some()
    }

    /// Rename a buffer when both old and new names are valid.
    pub fn rename(&mut self, old_name: &str, new_name: &str) -> bool {
        if old_name == new_name || new_name.is_empty() {
            return false;
        }

        if self.buffers.contains_key(new_name) {
            return false;
        }

        match self.buffers.remove(old_name) {
            Some(mut buffer) => {
                buffer.set_name(new_name.to_string());
                self.buffers.insert(new_name.to_string(), buffer);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BufferStore;

    #[test]
    fn open_creates_and_returns_buffer() {
        let mut store = BufferStore::new();
        let buffer = store.open("example");
        buffer.append("data".into());

        assert_eq!(store.get("example").unwrap().lines(), &["data".to_string()]);
    }

    #[test]
    fn list_and_is_empty_reflect_store_state() {
        let mut store = BufferStore::new();
        assert!(store.is_empty());
        store.open("first");
        store.open("second");
        let mut names = store.list();
        names.sort();
        assert_eq!(names, vec!["first".to_string(), "second".to_string()]);
        assert!(!store.is_empty());
    }

    #[test]
    fn delete_char_returns_position_when_successful() {
        let mut store = BufferStore::new();
        store.open("buf").append("abc".into());

        let pos = store.delete_char("buf", 0, 2).expect("delete succeed");
        assert_eq!(pos, (0, 1));
        assert_eq!(store.get("buf").unwrap().lines(), &["ac".to_string()]);
    }

    /// Removing a buffer evicts it while ignoring unknown names.
    #[test]
    fn remove_deletes_buffer_from_store() {
        let mut store = BufferStore::new();
        store.open("alpha");
        assert!(store.remove("alpha"));
        assert!(!store.remove("missing"));
        assert!(store.get("alpha").is_none());
    }

    #[test]
    fn reopening_marks_buffer_open_again() {
        let mut store = BufferStore::new();
        store.open("alpha").append("retain".into());
        store.mark_closed("alpha");

        let reopened = store.open("alpha");
        assert!(reopened.is_open());
        assert_eq!(reopened.lines(), &["retain".to_string()]);
    }

    #[test]
    fn list_all_includes_closed_buffers() {
        let mut store = BufferStore::new();
        store.open("alpha");
        store.open("beta");
        store.mark_closed("alpha");

        let names = store.list();
        assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn open_buffers_respects_open_state() {
        let mut store = BufferStore::new();
        store.open("alpha");
        store.open("beta");
        store.mark_closed("alpha");

        assert_eq!(store.open_buffers(), vec!["beta".to_string()]);
    }

    #[test]
    fn save_in_memory_marks_buffer_clean() {
        let mut store = BufferStore::new();
        store.open("alpha").append("dirty".into());
        assert!(store.is_dirty("alpha"));

        assert!(store.save_in_memory("alpha"));
        assert!(!store.is_dirty("alpha"));
        assert!(!store.save_in_memory("missing"));
    }
}
