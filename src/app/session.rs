use crate::events::{RequestSubtab, ResponseSubtab};
use crate::models::{CollectionItem, Request, Response};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestTabSession {
    pub id: usize,
    pub title: String,
    pub file_path: Option<PathBuf>,
    pub request: Request,
    pub url_cursor_pos: usize,
    pub request_subtab: RequestSubtab,
    pub response_subtab: ResponseSubtab,
    pub table_selected_row: usize,
    pub active_response: Option<Response>,
    pub error_message: Option<String>,
    pub is_loading: bool,
    pub response_scroll: u16,
    pub search_query: Option<String>,
    pub search_matches: Vec<usize>,
    pub selected_match_idx: usize,
}

impl RequestTabSession {
    pub fn new(id: usize, title: String, request: Request, file_path: Option<PathBuf>) -> Self {
        let url_cursor_pos = request.url.len();
        Self {
            id,
            title,
            file_path,
            request,
            url_cursor_pos,
            request_subtab: RequestSubtab::Query,
            response_subtab: ResponseSubtab::Body,
            table_selected_row: 0,
            active_response: None,
            error_message: None,
            is_loading: false,
            response_scroll: 0,
            search_query: None,
            search_matches: Vec::new(),
            selected_match_idx: 0,
        }
    }

    pub fn from_collection(id: usize, item: &CollectionItem) -> Self {
        Self::new(
            id,
            item.name.clone(),
            item.request.clone(),
            Some(item.path.clone()),
        )
    }

    pub fn blank(id: usize) -> Self {
        let req = Request::default();
        Self::new(id, "Untitled".to_string(), req, None)
    }

    pub fn update_search(&mut self, query: Option<String>) {
        self.search_query = query;
        self.search_matches.clear();
        self.selected_match_idx = 0;

        if let Some(q) = &self.search_query {
            if q.is_empty() {
                self.search_query = None;
                return;
            }
            let q_lower = q.to_lowercase();
            if let Some(res) = &self.active_response {
                let formatted = res.formatted_body();
                for (idx, line) in formatted.lines().enumerate() {
                    if line.to_lowercase().contains(&q_lower) {
                        self.search_matches.push(idx);
                    }
                }
            }

            if let Some(&first_match) = self.search_matches.first() {
                self.response_scroll = first_match as u16;
            }
        }
    }

    pub fn next_search_match(&mut self) {
        if !self.search_matches.is_empty() {
            self.selected_match_idx =
                (self.selected_match_idx + 1) % self.search_matches.len();
            self.response_scroll = self.search_matches[self.selected_match_idx] as u16;
        }
    }

    pub fn prev_search_match(&mut self) {
        if !self.search_matches.is_empty() {
            self.selected_match_idx =
                (self.selected_match_idx + self.search_matches.len() - 1)
                    % self.search_matches.len();
            self.response_scroll = self.search_matches[self.selected_match_idx] as u16;
        }
    }
}
