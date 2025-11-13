//! Serializable representation of a Buffer for persistence.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferSnapshot {
    pub name: String,
    pub lines: Vec<String>,
    pub requires_name: bool,
    pub is_open: bool,
    pub dirty: bool,
}

impl BufferSnapshot {
    pub fn new(
        name: String,
        lines: Vec<String>,
        requires_name: bool,
        is_open: bool,
        dirty: bool,
    ) -> Self {
        Self {
            name,
            lines,
            requires_name,
            is_open,
            dirty,
        }
    }
}
