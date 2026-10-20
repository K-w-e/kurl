use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub name: String,
    pub value: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl KeyValuePair {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            enabled: true,
        }
    }

    #[allow(dead_code)]
    pub fn with_enabled(name: impl Into<String>, value: impl Into<String>, enabled: bool) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            enabled,
        }
    }
}
