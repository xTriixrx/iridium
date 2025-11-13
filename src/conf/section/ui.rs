use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UiConfigSection {
    pub prompt_theme: Option<String>,
}
