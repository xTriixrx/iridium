use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ControlConfigSection {
    pub auto_save_interval_ms: Option<u64>,
    pub default_buffer_mode: Option<String>,
}
