use super::key_value::KeyValuePair;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub status_code: u16,
    pub status_text: String,
    pub duration_ms: u128,
    pub headers: Vec<KeyValuePair>,
    pub body: String,
    pub body_size_bytes: usize,
    pub content_type: Option<String>,
}

impl Response {
    #[allow(dead_code)]
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    #[allow(dead_code)]
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    #[allow(dead_code)]
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }

    pub fn formatted_body(&self) -> String {
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&self.body) {
            if let Ok(pretty) = serde_json::to_string_pretty(&json_val) {
                return pretty;
            }
        }
        self.body.clone()
    }
}
