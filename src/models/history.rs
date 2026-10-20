use super::http_method::HttpMethod;
use super::request::Request;
use super::response::Response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: usize,
    pub timestamp: String,
    pub method: HttpMethod,
    pub url: String,
    pub status_code: u16,
    pub duration_ms: u128,
    pub request: Request,
    pub response: Response,
}

impl HistoryItem {
    pub fn new(id: usize, request: Request, response: Response) -> Self {
        let now = std::time::SystemTime::now();
        let timestamp = match now.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => {
                let secs = d.as_secs();
                let hours = (secs / 3600) % 24;
                let minutes = (secs / 60) % 60;
                let seconds = secs % 60;
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            }
            Err(_) => "00:00:00".to_string(),
        };

        Self {
            id,
            timestamp,
            method: request.method,
            url: request.url.clone(),
            status_code: response.status_code,
            duration_ms: response.duration_ms,
            request,
            response,
        }
    }
}
