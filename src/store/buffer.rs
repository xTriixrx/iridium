use std::collections::HashMap;

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
}
