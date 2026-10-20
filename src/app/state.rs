use super::persistence::PersistenceManager;
use super::session::RequestTabSession;
use crate::config::Config;
use crate::events::{
    Action, DragTarget, Pane, RequestSubtab, ResponseSubtab, SidebarViewMode,
};
use crate::http::{copy_to_clipboard, from_curl, to_curl, HttpClient};
use crate::models::{
    AuthConfig, CollectionItem, Environment, HistoryItem, HttpMethod, KeyValuePair, Request,
    RequestBody,
};
use crate::ui::AppLayout;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::fs;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Modal {
    #[default]
    None,
    NewRequest {
        name_input: String,
        selected_method: HttpMethod,
        group_input: String,
        url_input: String,
        active_field: usize,
    },
    NewGroup {
        name_input: String,
    },
    EditTableRow {
        target_subtab: RequestSubtab,
        row_index: Option<usize>,
        key_input: String,
        value_input: String,
        active_field: usize,
    },
    ImportUniversal {
        input: String,
    },
    Export {
        format_idx: usize,
        path_input: String,
        active_field: usize,
    },
    SearchResponse {
        query: String,
    },
    CommandPalette {
        query: String,
        selected_idx: usize,
    },
    ConfirmDeleteRequest {
        name: String,
        path: PathBuf,
    },
    ConfirmDeleteGroup {
        group_name: String,
        path: PathBuf,
        req_count: usize,
    },
    SelectGroupToDelete {
        selected_idx: usize,
    },
    ConfirmClearHistory,
}

pub struct App {
    pub config: Config,
    pub focused_pane: Pane,
    pub sidebar_view_mode: SidebarViewMode,
    pub open_tabs: Vec<RequestTabSession>,
    pub active_tab_index: usize,
    pub next_tab_id: usize,
    pub collections: Vec<CollectionItem>,
    pub groups: Vec<String>,
    pub history: Vec<HistoryItem>,
    pub selected_history_index: usize,
    pub next_history_id: usize,
    pub environments: Vec<Environment>,
    pub active_env_index: usize,
    pub selected_collection: usize,
    pub selected_group: Option<String>,
    pub sidebar_scroll: usize,
    pub workspace_tabs_scroll: u16,
    pub term_width: u16,
    pub spinner_frame: usize,
    pub should_quit: bool,
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub http_client: HttpClient,
    pub sidebar_width: u16,
    pub request_split_percent: u16,
    pub active_drag: Option<DragTarget>,
    pub active_modal: Modal,
    pub status_flash_message: Option<String>,
    pub status_flash_ticks: usize,
    pub persistence: PersistenceManager,
}

impl App {
    pub fn new(config: Config, action_tx: mpsc::UnboundedSender<Action>) -> Result<Self> {
        let http_client = HttpClient::new(&config)?;
        let mut collections = CollectionItem::scan_directory(&config.collections_dir)?;
        let groups = CollectionItem::list_groups(&config.collections_dir).unwrap_or_default();
        let environments = Environment::scan_directory(&config.collections_dir);

        if collections.is_empty() {
            Self::seed_sample_collections(&config.collections_dir)?;
            collections = CollectionItem::scan_directory(&config.collections_dir)?;
        }

        let persistence = PersistenceManager::new(&config.persistence_dir);
        let saved_state = persistence.load_state().unwrap_or(None);
        let saved_history = persistence.load_history().unwrap_or_default();

        let (
            open_tabs,
            active_tab_index,
            sidebar_width,
            request_split_percent,
            active_env_index,
            sidebar_view_mode,
            next_tab_id,
        ) = if let Some(saved) = saved_state {
            let mut tabs = saved.open_tabs;
            for t in &mut tabs {
                t.is_loading = false;
            }
            if tabs.is_empty() {
                let initial_session = collections
                    .first()
                    .map(|item| RequestTabSession::from_collection(1, item))
                    .unwrap_or_else(|| RequestTabSession::blank(1));
                tabs.push(initial_session);
            }
            let active_idx = saved.active_tab_index.min(tabs.len().saturating_sub(1));
            let max_id = tabs.iter().map(|t| t.id).max().unwrap_or(0);
            (
                tabs,
                active_idx,
                saved.sidebar_width,
                saved.request_split_percent,
                saved.active_env_index,
                saved.sidebar_view_mode,
                max_id + 1,
            )
        } else {
            let initial_session = collections
                .first()
                .map(|item| RequestTabSession::from_collection(1, item))
                .unwrap_or_else(|| RequestTabSession::blank(1));
            (
                vec![initial_session],
                0,
                28,
                50,
                0,
                SidebarViewMode::Collections,
                2,
            )
        };

        let next_history_id = saved_history.iter().map(|h| h.id).max().unwrap_or(0) + 1;

        Ok(Self {
            config,
            focused_pane: Pane::UrlBar,
            sidebar_view_mode,
            open_tabs,
            active_tab_index,
            next_tab_id,
            collections,
            groups,
            history: saved_history,
            selected_history_index: 0,
            next_history_id,
            environments,
            active_env_index,
            selected_collection: 0,
            selected_group: None,
            sidebar_scroll: 0,
            workspace_tabs_scroll: 0,
            term_width: 120,
            spinner_frame: 0,
            should_quit: false,
            action_tx,
            http_client,
            sidebar_width,
            request_split_percent,
            active_drag: None,
            active_modal: Modal::None,
            status_flash_message: None,
            status_flash_ticks: 0,
            persistence,
        })
    }

    pub fn ensure_active_tab_visible(&mut self) {
        if self.open_tabs.is_empty() {
            self.workspace_tabs_scroll = 0;
            return;
        }
        let inner_width = self.term_width.saturating_sub(self.sidebar_width + 4).max(20);
        let (positions, _, plus_end) = crate::ui::tabs::compute_tab_widths(self);
        if let Some(&(start_x, end_x)) = positions.get(self.active_tab_index) {
            if start_x < self.workspace_tabs_scroll {
                self.workspace_tabs_scroll = start_x;
            } else if end_x + 1 > self.workspace_tabs_scroll + inner_width {
                self.workspace_tabs_scroll = (end_x + 1).saturating_sub(inner_width);
            }
        }
        if self.workspace_tabs_scroll > plus_end {
            self.workspace_tabs_scroll = plus_end.saturating_sub(inner_width);
        }
    }

    pub fn save_state(&self) -> Result<()> {
        let state = crate::app::AppSavedState {
            active_tab_index: self.active_tab_index,
            open_tabs: self.open_tabs.clone(),
            sidebar_width: self.sidebar_width,
            request_split_percent: self.request_split_percent,
            active_env_index: self.active_env_index,
            sidebar_view_mode: self.sidebar_view_mode,
        };
        self.persistence.save_state(&state)?;
        self.persistence.save_history(&self.history)?;
        Ok(())
    }

    pub fn current_session(&self) -> Option<&RequestTabSession> {
        self.open_tabs.get(self.active_tab_index)
    }

    pub fn current_session_mut(&mut self) -> Option<&mut RequestTabSession> {
        self.open_tabs.get_mut(self.active_tab_index)
    }

    pub fn active_environment(&self) -> Option<&Environment> {
        self.environments.get(self.active_env_index)
    }

    pub fn flash_message(&mut self, msg: impl Into<String>) {
        self.status_flash_message = Some(msg.into());
        self.status_flash_ticks = 120;
    }

    fn seed_sample_collections(dir: &PathBuf) -> Result<()> {
        fs::create_dir_all(dir)?;

        let get_sample = Request {
            name: "Get JSON".to_string(),
            description: Some("Fetch sample JSON data".to_string()),
            method: HttpMethod::Get,
            url: "https://httpbin.org/get".to_string(),
            auth: AuthConfig::None,
            headers: vec![
                KeyValuePair::new("Accept", "application/json"),
                KeyValuePair::new("User-Agent", "Posting-Rust/0.1.0"),
            ],
            params: vec![
                KeyValuePair::new("format", "json"),
                KeyValuePair::new("limit", "10"),
            ],
            path_params: Vec::new(),
            body: None,
        };
        let _ = get_sample.save_to_file(dir.join("get_sample.posting.yaml"));

        let _ = CollectionItem::create_group(dir, "auth");
        let post_sample = Request {
            name: "Create Item".to_string(),
            description: Some("Submit payload to endpoint".to_string()),
            method: HttpMethod::Post,
            url: "https://httpbin.org/post".to_string(),
            auth: AuthConfig::Bearer {
                token: "sample_jwt_token_xyz".to_string(),
            },
            headers: vec![
                KeyValuePair::new("Content-Type", "application/json"),
                KeyValuePair::new("Accept", "application/json"),
            ],
            params: Vec::new(),
            path_params: Vec::new(),
            body: Some(RequestBody {
                content: Some(
                    "{\n  \"name\": \"Posting Rust\",\n  \"status\": \"active\",\n  \"fast\": true\n}"
                        .to_string(),
                ),
            }),
        };
        let _ = post_sample.save_to_file(dir.join("auth").join("post_sample.posting.yaml"));

        let delay_sample = Request {
            name: "Delayed Response".to_string(),
            description: Some("Test async UI responsiveness".to_string()),
            method: HttpMethod::Get,
            url: "https://httpbin.org/delay/2".to_string(),
            auth: AuthConfig::None,
            headers: Vec::new(),
            params: Vec::new(),
            path_params: Vec::new(),
            body: None,
        };
        let _ = delay_sample.save_to_file(dir.join("delay_sample.posting.yaml"));

        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.focused_pane != Pane::UrlBar {
                return Some(Action::CopyAsCurl);
            }
        }

        if self.active_modal != Modal::None {
            match key.code {
                KeyCode::Esc => return Some(Action::CloseModal),
                KeyCode::Enter => return Some(Action::SubmitModal),
                KeyCode::Tab => return Some(Action::ModalNextField),
                KeyCode::BackTab => return Some(Action::ModalPrevField),
                KeyCode::Up => {
                    match &mut self.active_modal {
                        Modal::CommandPalette { selected_idx, .. } => {
                            if *selected_idx > 0 {
                                *selected_idx -= 1;
                            }
                        }
                        Modal::SelectGroupToDelete { selected_idx } => {
                            if *selected_idx > 0 {
                                *selected_idx -= 1;
                            }
                        }
                        _ => {}
                    }
                    return None;
                }
                KeyCode::Down => {
                    match &mut self.active_modal {
                        Modal::CommandPalette {
                            query,
                            selected_idx,
                        } => {
                            let q_lower = query.trim().to_lowercase();
                            let count = self
                                .collections
                                .iter()
                                .filter(|item| {
                                    if q_lower.is_empty() {
                                        true
                                    } else {
                                        item.name.to_lowercase().contains(&q_lower)
                                            || item
                                                .request
                                                .method
                                                .as_str()
                                                .to_lowercase()
                                                .contains(&q_lower)
                                            || item.request.url.to_lowercase().contains(&q_lower)
                                            || item
                                                .group
                                                .as_deref()
                                                .unwrap_or("")
                                                .to_lowercase()
                                                .contains(&q_lower)
                                    }
                                })
                                .count();
                            if count > 0 && *selected_idx + 1 < count {
                                *selected_idx += 1;
                            }
                        }
                        Modal::SelectGroupToDelete { selected_idx } => {
                            if !self.groups.is_empty() && *selected_idx + 1 < self.groups.len() {
                                *selected_idx += 1;
                            }
                        }
                        _ => {}
                    }
                    return None;
                }
                KeyCode::Left => {
                    if let Modal::NewRequest {
                        active_field: 1,
                        selected_method,
                        ..
                    } = &mut self.active_modal
                    {
                        *selected_method = selected_method.prev();
                    } else if let Modal::Export {
                        active_field: 0,
                        format_idx,
                        ..
                    } = &mut self.active_modal
                    {
                        *format_idx = (*format_idx + 4) % 5;
                    }
                    return None;
                }
                KeyCode::Right => {
                    if let Modal::NewRequest {
                        active_field: 1,
                        selected_method,
                        ..
                    } = &mut self.active_modal
                    {
                        *selected_method = selected_method.next();
                    } else if let Modal::Export {
                        active_field: 0,
                        format_idx,
                        ..
                    } = &mut self.active_modal
                    {
                        *format_idx = (*format_idx + 1) % 5;
                    }
                    return None;
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    if let Modal::NewRequest {
                        active_field: 1,
                        selected_method,
                        ..
                    } = &mut self.active_modal
                    {
                        *selected_method = selected_method.next();
                        return None;
                    }
                }
                KeyCode::Char(c) => {
                    match &mut self.active_modal {
                        Modal::NewRequest {
                            name_input,
                            url_input,
                            group_input,
                            active_field,
                            ..
                        } => match *active_field {
                            0 => name_input.push(c),
                            2 => url_input.push(c),
                            3 => group_input.push(c),
                            _ => {}
                        },
                        Modal::NewGroup { name_input } => {
                            name_input.push(c);
                        }
                        Modal::EditTableRow {
                            key_input,
                            value_input,
                            active_field,
                            ..
                        } => match *active_field {
                            0 => key_input.push(c),
                            _ => value_input.push(c),
                        },
                        Modal::ImportUniversal { input } => {
                            input.push(c);
                        }
                        Modal::Export {
                            active_field,
                            format_idx,
                            path_input,
                        } => {
                            if *active_field == 0 {
                                if let Some(d) = c.to_digit(10) {
                                    if d >= 1 && d <= 5 {
                                        *format_idx = (d - 1) as usize;
                                    }
                                }
                            } else {
                                path_input.push(c);
                            }
                        }
                        Modal::SearchResponse { query } => {
                            query.push(c);
                            let q = query.clone();
                            if let Some(session) = self.current_session_mut() {
                                session.update_search(Some(q));
                            }
                        }
                        Modal::CommandPalette {
                            query,
                            selected_idx,
                        } => {
                            query.push(c);
                            *selected_idx = 0;
                        }
                        _ => {}
                    }
                    return None;
                }
                KeyCode::Backspace => {
                    match &mut self.active_modal {
                        Modal::NewRequest {
                            name_input,
                            url_input,
                            group_input,
                            active_field,
                            ..
                        } => match *active_field {
                            0 => {
                                name_input.pop();
                            }
                            2 => {
                                url_input.pop();
                            }
                            3 => {
                                group_input.pop();
                            }
                            _ => {}
                        },
                        Modal::NewGroup { name_input } => {
                            name_input.pop();
                        }
                        Modal::EditTableRow {
                            key_input,
                            value_input,
                            active_field,
                            ..
                        } => match *active_field {
                            0 => {
                                key_input.pop();
                            }
                            _ => {
                                value_input.pop();
                            }
                        },
                        Modal::ImportUniversal { input } => {
                            input.pop();
                        }
                        Modal::Export {
                            active_field,
                            path_input,
                            ..
                        } => {
                            if *active_field == 1 {
                                path_input.pop();
                            }
                        }
                        Modal::SearchResponse { query } => {
                            query.pop();
                            let q = if query.is_empty() {
                                None
                            } else {
                                Some(query.clone())
                            };
                            if let Some(session) = self.current_session_mut() {
                                session.update_search(q);
                            }
                        }
                        Modal::CommandPalette {
                            query,
                            selected_idx,
                        } => {
                            query.pop();
                            *selected_idx = 0;
                        }
                        _ => {}
                    }
                    return None;
                }
                _ => return None,
            }
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('w') => return Some(Action::CloseCurrentTab),
                KeyCode::Char('t') => return Some(Action::NewTab),
                KeyCode::Char('e') => return Some(Action::NextEnvironment),
                KeyCode::Char('i') => return Some(Action::OpenImportCurlModal),
                KeyCode::Tab | KeyCode::PageDown => return Some(Action::NextOpenTab),
                KeyCode::BackTab | KeyCode::PageUp => return Some(Action::PrevOpenTab),
                KeyCode::Left => return Some(Action::ResizeSidebar(-2)),
                KeyCode::Right => return Some(Action::ResizeSidebar(2)),
                KeyCode::Up => return Some(Action::ResizeVerticalSplit(-4)),
                KeyCode::Down => return Some(Action::ResizeVerticalSplit(4)),
                KeyCode::Char('r') => return Some(Action::SendRequest),
                KeyCode::Char('s') => return Some(Action::SaveRequest),
                KeyCode::Char('n') => return Some(Action::OpenNewRequestModal),
                KeyCode::Char('p') => return Some(Action::OpenCommandPalette),
                _ => {}
            }
        }

        if key.modifiers.contains(KeyModifiers::ALT) {
            if let KeyCode::Char(c) = key.code {
                if let Some(digit) = c.to_digit(10) {
                    if digit >= 1 && digit <= 9 {
                        return Some(Action::SwitchOpenTab((digit - 1) as usize));
                    }
                }
            }
        }

        if key.code == KeyCode::F(5) {
            return Some(Action::SendRequest);
        }

        match self.focused_pane {
            Pane::UrlBar => match key.code {
                KeyCode::Enter => Some(Action::SendRequest),
                KeyCode::Tab => Some(Action::FocusNext),
                KeyCode::BackTab => Some(Action::FocusPrev),
                KeyCode::Esc => Some(Action::SwitchFocus(Pane::Sidebar)),
                KeyCode::Char(c) => Some(Action::InputChar(c)),
                KeyCode::Backspace => Some(Action::Backspace),
                KeyCode::Delete => Some(Action::Delete),
                KeyCode::Left => Some(Action::CursorLeft),
                KeyCode::Right => Some(Action::CursorRight),
                _ => None,
            },
            Pane::Sidebar => match key.code {
                KeyCode::Tab => Some(Action::FocusNext),
                KeyCode::BackTab => Some(Action::FocusPrev),
                KeyCode::Up | KeyCode::Char('k') => match self.sidebar_view_mode {
                    SidebarViewMode::Collections => Some(Action::SelectPrevCollection),
                    SidebarViewMode::History => {
                        if self.selected_history_index > 0 {
                            self.selected_history_index -= 1;
                        }
                        None
                    }
                },
                KeyCode::Down | KeyCode::Char('j') => match self.sidebar_view_mode {
                    SidebarViewMode::Collections => Some(Action::SelectNextCollection),
                    SidebarViewMode::History => {
                        if !self.history.is_empty()
                            && self.selected_history_index + 1 < self.history.len()
                        {
                            self.selected_history_index += 1;
                        }
                        None
                    }
                },
                KeyCode::Enter => match self.sidebar_view_mode {
                    SidebarViewMode::Collections => Some(Action::LoadSelectedCollection),
                    SidebarViewMode::History => Some(Action::LoadSelectedHistory),
                },
                KeyCode::Char('h') => Some(Action::ToggleSidebarView),
                KeyCode::Char('n') => Some(Action::OpenNewRequestModal),
                KeyCode::Char('N') | KeyCode::Char('g') => Some(Action::OpenNewGroupModal),
                KeyCode::Char('d') | KeyCode::Delete => match self.sidebar_view_mode {
                    SidebarViewMode::Collections => Some(Action::OpenDeleteRequestModal),
                    SidebarViewMode::History => Some(Action::DeleteSelectedHistory),
                },
                KeyCode::Char('D') => match self.sidebar_view_mode {
                    SidebarViewMode::Collections => Some(Action::OpenDeleteGroupModal),
                    SidebarViewMode::History => Some(Action::OpenClearHistoryModal),
                },
                KeyCode::Char('C') => match self.sidebar_view_mode {
                    SidebarViewMode::Collections => None,
                    SidebarViewMode::History => Some(Action::OpenClearHistoryModal),
                },
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Char('1') => Some(Action::SwitchFocus(Pane::Sidebar)),
                KeyCode::Char('2') => Some(Action::SwitchFocus(Pane::UrlBar)),
                KeyCode::Char('3') => Some(Action::SwitchFocus(Pane::RequestTabs)),
                KeyCode::Char('4') => Some(Action::SwitchFocus(Pane::ResponseViewer)),
                _ => None,
            },
            Pane::WorkspaceTabs => match key.code {
                KeyCode::Tab => Some(Action::FocusNext),
                KeyCode::BackTab => Some(Action::FocusPrev),
                KeyCode::Left | KeyCode::Char('h') => Some(Action::PrevOpenTab),
                KeyCode::Right | KeyCode::Char('l') => Some(Action::NextOpenTab),
                KeyCode::Char('w') => Some(Action::CloseCurrentTab),
                KeyCode::Char('t') => Some(Action::NewTab),
                KeyCode::Enter => Some(Action::SwitchFocus(Pane::UrlBar)),
                KeyCode::Char('q') => Some(Action::Quit),
                _ => None,
            },
            Pane::RequestTabs => match key.code {
                KeyCode::Tab => Some(Action::FocusNext),
                KeyCode::BackTab => Some(Action::FocusPrev),
                KeyCode::Left | KeyCode::Char('h') => Some(Action::PrevRequestSubtab),
                KeyCode::Right | KeyCode::Char('l') => Some(Action::NextRequestSubtab),
                KeyCode::Enter | KeyCode::Down => Some(Action::SwitchFocus(Pane::RequestEditor)),
                KeyCode::Char('m') => Some(Action::NextMethod),
                KeyCode::Char('M') => Some(Action::PrevMethod),
                KeyCode::Char('a') => Some(Action::AddTableRow),
                KeyCode::Char('e') => Some(Action::EditTableRow),
                KeyCode::Char('d') => Some(Action::DeleteTableRow),
                KeyCode::Char('q') => Some(Action::Quit),
                _ => None,
            },
            Pane::RequestEditor => {
                let subtab = self
                    .current_session()
                    .map(|s| s.request_subtab)
                    .unwrap_or_default();
                match subtab {
                    RequestSubtab::Query | RequestSubtab::Headers | RequestSubtab::Path => {
                        match key.code {
                            KeyCode::Tab => Some(Action::FocusNext),
                            KeyCode::BackTab => Some(Action::FocusPrev),
                            KeyCode::Up | KeyCode::Char('k') => Some(Action::SelectPrevRow),
                            KeyCode::Down | KeyCode::Char('j') => Some(Action::SelectNextRow),
                            KeyCode::Char(' ') => Some(Action::ToggleRow),
                            KeyCode::Char('a') => Some(Action::AddTableRow),
                            KeyCode::Char('e') | KeyCode::Enter => Some(Action::EditTableRow),
                            KeyCode::Char('d') | KeyCode::Delete => Some(Action::DeleteTableRow),
                            KeyCode::Char('q') => Some(Action::Quit),
                            _ => None,
                        }
                    }
                    RequestSubtab::Auth => match key.code {
                        KeyCode::Tab => Some(Action::FocusNext),
                        KeyCode::BackTab => Some(Action::FocusPrev),
                        KeyCode::Char('m') => Some(Action::NextAuthType),
                        KeyCode::Char('M') => Some(Action::PrevAuthType),
                        KeyCode::Char('e') | KeyCode::Enter => Some(Action::EditTableRow),
                        KeyCode::Char('q') => Some(Action::Quit),
                        _ => None,
                    },
                    RequestSubtab::Body => match key.code {
                        KeyCode::Tab => Some(Action::FocusNext),
                        KeyCode::BackTab => Some(Action::FocusPrev),
                        KeyCode::Esc => Some(Action::SwitchFocus(Pane::RequestTabs)),
                        KeyCode::Char(c) => Some(Action::InputChar(c)),
                        KeyCode::Backspace => Some(Action::Backspace),
                        KeyCode::Enter => Some(Action::InputChar('\n')),
                        _ => None,
                    },
                }
            }
            Pane::ResponseViewer => match key.code {
                KeyCode::Tab => Some(Action::FocusNext),
                KeyCode::BackTab => Some(Action::FocusPrev),
                KeyCode::Up | KeyCode::Char('k') => Some(Action::ScrollResponseUp),
                KeyCode::Down | KeyCode::Char('j') => Some(Action::ScrollResponseDown),
                KeyCode::Left | KeyCode::Char('h') => Some(Action::PrevResponseSubtab),
                KeyCode::Right | KeyCode::Char('l') => Some(Action::NextResponseSubtab),
                KeyCode::Char('/') => Some(Action::OpenSearchModal),
                KeyCode::Char('n') => Some(Action::NextSearchMatch),
                KeyCode::Char('N') => Some(Action::PrevSearchMatch),
                KeyCode::Esc => {
                    if self
                        .current_session()
                        .and_then(|s| s.search_query.as_ref())
                        .is_some()
                    {
                        Some(Action::ClearSearch)
                    } else {
                        Some(Action::SwitchFocus(Pane::RequestTabs))
                    }
                }
                KeyCode::Char('y') => Some(Action::CopyResponseBody),
                KeyCode::Char('c') => Some(Action::CopyAsCurl),
                KeyCode::Char('r') => Some(Action::SendRequest),
                KeyCode::Char('q') => Some(Action::Quit),
                _ => None,
            },
            _ => match key.code {
                KeyCode::Tab => Some(Action::FocusNext),
                KeyCode::BackTab => Some(Action::FocusPrev),
                KeyCode::Char('q') => Some(Action::Quit),
                _ => None,
            },
        }
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent, area: Rect) -> Option<Action> {
        let layout = AppLayout::compute(area, self.sidebar_width, self.request_split_percent);
        let col = mouse.column;
        let row = mouse.row;

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.active_modal != Modal::None {
                    if let Some(m_rect) = crate::ui::modal::modal_rect(&self.active_modal, area) {
                        if !AppLayout::contains(m_rect, col, row) {
                            return Some(Action::CloseModal);
                        }
                        let button_row = m_rect.y + m_rect.height.saturating_sub(2);
                        if row == button_row || row == button_row.saturating_sub(1) {
                            let rel_x = col.saturating_sub(m_rect.x);
                            if rel_x < m_rect.width / 2 {
                                return Some(Action::SubmitModal);
                            } else {
                                return Some(Action::CloseModal);
                            }
                        }
                        match &mut self.active_modal {
                            Modal::NewRequest {
                                active_field,
                                selected_method,
                                ..
                            } => {
                                let inner_y = row.saturating_sub(m_rect.y + 1);
                                if inner_y < 3 {
                                    *active_field = 0;
                                } else if inner_y < 6 {
                                    *active_field = 1;
                                    *selected_method = selected_method.next();
                                } else if inner_y < 9 {
                                    *active_field = 2;
                                } else if inner_y < 12 {
                                    *active_field = 3;
                                }
                                return Some(Action::Render);
                            }
                            Modal::EditTableRow { active_field, .. } => {
                                let inner_y = row.saturating_sub(m_rect.y + 1);
                                if inner_y < 3 {
                                    *active_field = 0;
                                } else if inner_y < 6 {
                                    *active_field = 1;
                                }
                                return Some(Action::Render);
                            }
                            Modal::CommandPalette {
                                selected_idx,
                                query: _,
                            } => {
                                let list_start_y = m_rect.y + 4;
                                if row >= list_start_y && row < button_row {
                                    let clicked_offset = (row - list_start_y) as usize;
                                    *selected_idx = clicked_offset;
                                    return Some(Action::SubmitModal);
                                }
                            }
                            Modal::SelectGroupToDelete { selected_idx } => {
                                let list_start_y = m_rect.y + 1;
                                if row >= list_start_y && row < button_row {
                                    let clicked_offset = (row - list_start_y) as usize;
                                    *selected_idx = clicked_offset;
                                    return Some(Action::SubmitModal);
                                }
                            }
                            Modal::Export {
                                active_field,
                                format_idx,
                                ..
                            } => {
                                let inner_y = row.saturating_sub(m_rect.y + 1);
                                if inner_y < 3 {
                                    *active_field = 0;
                                    *format_idx = (*format_idx + 1) % 5;
                                } else if inner_y < 6 {
                                    *active_field = 1;
                                }
                                return Some(Action::Render);
                            }
                            _ => {}
                        }
                    }
                    return None;
                }

                if AppLayout::contains(layout.footer, col, row) {
                    let rel_x = col.saturating_sub(layout.footer.x);
                    let (_, targets) = crate::ui::footer::build_footer(self, layout.footer.width);
                    for target in targets {
                        if rel_x >= target.start_x && rel_x < target.end_x {
                            return Some(target.action);
                        }
                    }
                    return None;
                }

                if AppLayout::contains(layout.sidebar_divider, col, row) {
                    self.active_drag = Some(DragTarget::SidebarWidth);
                    return Some(Action::StartDrag(DragTarget::SidebarWidth));
                }

                if AppLayout::contains(layout.horizontal_divider, col, row) {
                    self.active_drag = Some(DragTarget::RequestResponseSplit);
                    return Some(Action::StartDrag(DragTarget::RequestResponseSplit));
                }

                if AppLayout::contains(layout.sidebar_new_buttons, col, row) {
                    self.focused_pane = Pane::Sidebar;
                    let rel_x = col.saturating_sub(layout.sidebar_new_buttons.x + 1);
                    match self.sidebar_view_mode {
                        SidebarViewMode::Collections => {
                            if rel_x < 13 {
                                self.sidebar_view_mode = SidebarViewMode::Collections;
                            } else if rel_x < 28 {
                                self.sidebar_view_mode = SidebarViewMode::History;
                            } else if rel_x < 35 {
                                return Some(Action::OpenNewRequestModal);
                            } else if rel_x < 42 {
                                return Some(Action::OpenNewGroupModal);
                            } else {
                                return Some(Action::OpenDeleteGroupModal);
                            }
                        }
                        SidebarViewMode::History => {
                            if rel_x < 13 {
                                self.sidebar_view_mode = SidebarViewMode::Collections;
                            } else if rel_x < 28 {
                                self.sidebar_view_mode = SidebarViewMode::History;
                            } else if rel_x < 34 {
                                return Some(Action::DeleteSelectedHistory);
                            } else {
                                return Some(Action::OpenClearHistoryModal);
                            }
                        }
                    }
                    return Some(Action::SwitchFocus(Pane::Sidebar));
                }

                if AppLayout::contains(layout.sidebar_list, col, row) {
                    self.focused_pane = Pane::Sidebar;
                    match self.sidebar_view_mode {
                        SidebarViewMode::Collections => {
                            if row > layout.sidebar_list.y
                                && row
                                    <= layout
                                        .sidebar_list
                                        .y
                                        + layout.sidebar_list.height.saturating_sub(2)
                            {
                                let clicked_row = (row - layout.sidebar_list.y - 1) as usize
                                    + self.sidebar_scroll;
                                let entries = crate::ui::sidebar::build_sidebar_entries(
                                    &self.collections,
                                    &self.groups,
                                );
                                if let Some(entry) = entries.get(clicked_row) {
                                    match entry {
                                        crate::ui::sidebar::SidebarEntry::RequestItem {
                                             collection_idx,
                                        } => {
                                            self.selected_collection = *collection_idx;
                                            return Some(Action::LoadSelectedCollection);
                                        }
                                        crate::ui::sidebar::SidebarEntry::GroupHeader {
                                            name,
                                            is_root,
                                        } => {
                                            if !*is_root {
                                                self.selected_group = Some(name.clone());
                                                self.flash_message(format!(
                                                    "Selected group '{}'",
                                                    name
                                                ));
                                            }
                                        }
                                        crate::ui::sidebar::SidebarEntry::EmptyGroupNotice => {}
                                    }
                                }
                            }
                        }
                        SidebarViewMode::History => {
                            if row > layout.sidebar_list.y
                                && row
                                    <= layout
                                        .sidebar_list
                                        .y
                                        + layout.sidebar_list.height.saturating_sub(2)
                            {
                                let clicked_idx = (row - layout.sidebar_list.y - 1) as usize
                                    + self.sidebar_scroll;
                                if clicked_idx < self.history.len() {
                                    self.selected_history_index = clicked_idx;
                                    return Some(Action::LoadSelectedHistory);
                                }
                            }
                        }
                    }
                    return Some(Action::SwitchFocus(Pane::Sidebar));
                }

                if AppLayout::contains(layout.workspace_tabs, col, row) {
                    self.focused_pane = Pane::WorkspaceTabs;
                    if row == layout.workspace_tabs.y {
                        if col < layout.workspace_tabs.x + 20 && self.workspace_tabs_scroll > 0 {
                            return Some(Action::ScrollTabsLeft);
                        } else if col > layout.workspace_tabs.x + 20 {
                            return Some(Action::ScrollTabsRight);
                        }
                    }
                    let click_rel_x = (col.saturating_sub(layout.workspace_tabs.x + 1))
                        + self.workspace_tabs_scroll;
                    let (tab_positions, plus_start, plus_end) =
                        crate::ui::tabs::compute_tab_widths(self);

                    for (idx, (start_x, end_x)) in tab_positions.iter().enumerate() {
                        if click_rel_x >= *start_x && click_rel_x < *end_x {
                            if click_rel_x >= end_x.saturating_sub(3) {
                                return Some(Action::CloseOpenTab(idx));
                            } else {
                                return Some(Action::SwitchOpenTab(idx));
                            }
                        }
                    }

                    if click_rel_x >= plus_start && click_rel_x < plus_end {
                        return Some(Action::NewTab);
                    }
                    return Some(Action::SwitchFocus(Pane::WorkspaceTabs));
                }

                if AppLayout::contains(layout.url_bar, col, row) {
                    self.focused_pane = Pane::UrlBar;
                    if let Some(session) = self.current_session() {
                        let method_width = session.request.method.as_str().len() as u16 + 2;
                        let method_start = layout.url_bar.x + 1;
                        let method_end = method_start + method_width;

                        if col >= method_start && col <= method_end {
                            return Some(Action::NextMethod);
                        }

                        let send_btn_w = 10u16;
                        let send_start =
                            layout.url_bar.x + layout.url_bar.width.saturating_sub(send_btn_w + 1);
                        if col >= send_start {
                            return Some(Action::SendRequest);
                        }

                        let url_start = method_end + 1;
                        if col >= url_start && col < send_start {
                            let target_pos = (col - url_start) as usize;
                            if let Some(s) = self.current_session_mut() {
                                s.url_cursor_pos = target_pos.min(s.request.url.len());
                            }
                        }
                    }
                    return Some(Action::SwitchFocus(Pane::UrlBar));
                }

                if AppLayout::contains(layout.request_tabs, col, row) {
                    self.focused_pane = Pane::RequestTabs;
                    let rel_x = col.saturating_sub(layout.request_tabs.x + 1);
                    if let Some(session) = self.current_session_mut() {
                        if rel_x < 14 {
                            session.request_subtab = RequestSubtab::Query;
                        } else if rel_x < 30 {
                            session.request_subtab = RequestSubtab::Headers;
                        } else if rel_x < 40 {
                            session.request_subtab = RequestSubtab::Auth;
                        } else if rel_x < 50 {
                            session.request_subtab = RequestSubtab::Body;
                        } else {
                            session.request_subtab = RequestSubtab::Path;
                        }
                        session.table_selected_row = 0;
                    }
                    return Some(Action::SwitchFocus(Pane::RequestTabs));
                }

                if AppLayout::contains(layout.request_editor, col, row) {
                    self.focused_pane = Pane::RequestEditor;
                    if let Some(session) = self.current_session_mut() {
                        match session.request_subtab {
                            RequestSubtab::Query
                            | RequestSubtab::Headers
                            | RequestSubtab::Path => {
                                let header_offset = 2;
                                if row >= layout.request_editor.y + header_offset {
                                    let row_idx =
                                        (row - layout.request_editor.y - header_offset) as usize;
                                    let count = match session.request_subtab {
                                        RequestSubtab::Query => session.request.params.len(),
                                        RequestSubtab::Headers => session.request.headers.len(),
                                        RequestSubtab::Path => session.request.path_params.len(),
                                        _ => 0,
                                    };
                                    if row_idx < count {
                                        session.table_selected_row = row_idx;
                                        if col <= layout.request_editor.x + 8 {
                                            return Some(Action::ToggleRow);
                                        }
                                    }
                                }
                            }
                            RequestSubtab::Auth => {
                                return Some(Action::NextAuthType);
                            }
                            RequestSubtab::Body => {}
                        }
                    }
                    return Some(Action::SwitchFocus(Pane::RequestEditor));
                }

                if AppLayout::contains(layout.response_tabs, col, row) {
                    self.focused_pane = Pane::ResponseTabs;
                    let rel_x = col.saturating_sub(layout.response_tabs.x + 1);
                    if let Some(session) = self.current_session_mut() {
                        if rel_x < 12 {
                            session.response_subtab = ResponseSubtab::Body;
                        } else {
                            session.response_subtab = ResponseSubtab::Headers;
                        }
                        session.response_scroll = 0;
                    }
                    return Some(Action::SwitchFocus(Pane::ResponseTabs));
                }

                if AppLayout::contains(layout.response_content, col, row) {
                    return Some(Action::SwitchFocus(Pane::ResponseViewer));
                }

                None
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(target) = self.active_drag {
                    match target {
                        DragTarget::SidebarWidth => {
                            self.sidebar_width = col.clamp(16, area.width.saturating_sub(25));
                            return Some(Action::Render);
                        }
                        DragTarget::RequestResponseSplit => {
                            let top_y = layout.url_bar.y;
                            let total_h = area.height.saturating_sub(top_y + 1);
                            if total_h > 0 {
                                let rel =
                                    ((row.saturating_sub(top_y) as f32 / total_h as f32) * 100.0)
                                        as u16;
                                self.request_split_percent = rel.clamp(20, 80);
                            }
                            return Some(Action::Render);
                        }
                    }
                }
                None
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.active_drag.is_some() {
                    self.active_drag = None;
                    return Some(Action::EndDrag);
                }
                None
            }
            MouseEventKind::ScrollUp => {
                if AppLayout::contains(layout.response_content, col, row) {
                    Some(Action::ScrollResponseUp)
                } else if AppLayout::contains(layout.workspace_tabs, col, row) {
                    Some(Action::ScrollTabsLeft)
                } else if AppLayout::contains(layout.sidebar_list, col, row) {
                    Some(Action::ScrollSidebarUp)
                } else if AppLayout::contains(layout.request_editor, col, row) {
                    Some(Action::SelectPrevRow)
                } else {
                    None
                }
            }
            MouseEventKind::ScrollDown => {
                if AppLayout::contains(layout.response_content, col, row) {
                    Some(Action::ScrollResponseDown)
                } else if AppLayout::contains(layout.workspace_tabs, col, row) {
                    Some(Action::ScrollTabsRight)
                } else if AppLayout::contains(layout.sidebar_list, col, row) {
                    Some(Action::ScrollSidebarDown)
                } else if AppLayout::contains(layout.request_editor, col, row) {
                    Some(Action::SelectNextRow)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn update(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Quit => {
                self.should_quit = true;
                let _ = self.save_state();
            }
            Action::Tick => {
                self.spinner_frame = (self.spinner_frame + 1) % 4;
                if self.status_flash_ticks > 0 {
                    self.status_flash_ticks -= 1;
                    if self.status_flash_ticks == 0 {
                        self.status_flash_message = None;
                    }
                }
            }
            Action::Render => {}
            Action::FocusNext => {
                self.focused_pane = self.focused_pane.next();
            }
            Action::FocusPrev => {
                self.focused_pane = self.focused_pane.prev();
            }
            Action::SwitchFocus(pane) => {
                self.focused_pane = pane;
            }
            Action::NextOpenTab => {
                if !self.open_tabs.is_empty() {
                    self.active_tab_index = (self.active_tab_index + 1) % self.open_tabs.len();
                    self.ensure_active_tab_visible();
                }
            }
            Action::PrevOpenTab => {
                if !self.open_tabs.is_empty() {
                    self.active_tab_index =
                        (self.active_tab_index + self.open_tabs.len() - 1) % self.open_tabs.len();
                    self.ensure_active_tab_visible();
                }
            }
            Action::SwitchOpenTab(idx) => {
                if idx < self.open_tabs.len() {
                    self.active_tab_index = idx;
                    self.ensure_active_tab_visible();
                }
            }
            Action::CloseOpenTab(idx) => {
                if idx < self.open_tabs.len() {
                    if self.open_tabs.len() > 1 {
                        self.open_tabs.remove(idx);
                        if self.active_tab_index >= self.open_tabs.len() {
                            self.active_tab_index = self.open_tabs.len() - 1;
                        }
                    } else {
                        self.next_tab_id += 1;
                        self.open_tabs[0] = RequestTabSession::blank(self.next_tab_id);
                    }
                    self.ensure_active_tab_visible();
                    let _ = self.save_state();
                }
            }
            Action::CloseCurrentTab => {
                let idx = self.active_tab_index;
                let _ = self.update(Action::CloseOpenTab(idx));
            }
            Action::NewTab => {
                self.next_tab_id += 1;
                let session = RequestTabSession::blank(self.next_tab_id);
                self.open_tabs.push(session);
                self.active_tab_index = self.open_tabs.len() - 1;
                self.focused_pane = Pane::UrlBar;
                self.ensure_active_tab_visible();
                let _ = self.save_state();
            }
            Action::NextRequestSubtab => {
                if let Some(session) = self.current_session_mut() {
                    session.request_subtab = session.request_subtab.next();
                    session.table_selected_row = 0;
                }
            }
            Action::PrevRequestSubtab => {
                if let Some(session) = self.current_session_mut() {
                    session.request_subtab = session.request_subtab.prev();
                    session.table_selected_row = 0;
                }
            }
            Action::NextResponseSubtab => {
                if let Some(session) = self.current_session_mut() {
                    session.response_subtab = session.response_subtab.next();
                    session.response_scroll = 0;
                }
            }
            Action::PrevResponseSubtab => {
                if let Some(session) = self.current_session_mut() {
                    session.response_subtab = session.response_subtab.prev();
                    session.response_scroll = 0;
                }
            }
            Action::NextMethod => {
                if let Some(session) = self.current_session_mut() {
                    session.request.method = session.request.method.next();
                }
            }
            Action::PrevMethod => {
                if let Some(session) = self.current_session_mut() {
                    session.request.method = session.request.method.prev();
                }
            }
            Action::SendRequest => {
                let env_opt = self.environments.get(self.active_env_index).cloned();
                let to_send = if let Some(session) = self.current_session_mut() {
                    if !session.is_loading {
                        session.is_loading = true;
                        session.error_message = None;

                        let resolved = if let Some(env) = &env_opt {
                            env.resolve_request(&session.request)
                        } else {
                            session.request.clone()
                        };

                        Some((session.id, resolved))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some((tab_id, request)) = to_send {
                    let client = self.http_client.clone();
                    let tx = self.action_tx.clone();

                    tokio::spawn(async move {
                        match client.execute(&request).await {
                            Ok(res) => {
                                let _ = tx.send(Action::ResponseReceived(tab_id, res));
                            }
                            Err(err) => {
                                let _ = tx.send(Action::RequestFailed(tab_id, err.to_string()));
                            }
                        }
                    });
                }
            }
            Action::RequestStarted => {}
            Action::ResponseReceived(tab_id, res) => {
                if let Some(session) = self.open_tabs.iter_mut().find(|s| s.id == tab_id) {
                    session.is_loading = false;
                    session.active_response = Some(res.clone());
                    session.error_message = None;
                    session.response_scroll = 0;

                    let history_entry =
                        HistoryItem::new(self.next_history_id, session.request.clone(), res);
                    self.next_history_id += 1;
                    self.history.insert(0, history_entry);
                    if self.history.len() > 50 {
                        self.history.truncate(50);
                    }
                    let _ = self.save_state();
                }
                self.focused_pane = Pane::ResponseViewer;
            }
            Action::RequestFailed(tab_id, err) => {
                if let Some(session) = self.open_tabs.iter_mut().find(|s| s.id == tab_id) {
                    session.is_loading = false;
                    session.error_message = Some(err);
                }
            }
            Action::InputChar(c) => match self.focused_pane {
                Pane::UrlBar => {
                    if let Some(session) = self.current_session_mut() {
                        session.request.url.insert(session.url_cursor_pos, c);
                        session.url_cursor_pos += 1;
                    }
                }
                Pane::RequestEditor => {
                    if let Some(session) = self.current_session_mut() {
                        if session.request_subtab == RequestSubtab::Body {
                            let body = session.request.body.get_or_insert_with(Default::default);
                            let content = body.content.get_or_insert_with(String::new);
                            content.push(c);
                        }
                    }
                }
                _ => {}
            },
            Action::Backspace => match self.focused_pane {
                Pane::UrlBar => {
                    if let Some(session) = self.current_session_mut() {
                        if session.url_cursor_pos > 0 && !session.request.url.is_empty() {
                            session.url_cursor_pos -= 1;
                            session.request.url.remove(session.url_cursor_pos);
                        }
                    }
                }
                Pane::RequestEditor => {
                    if let Some(session) = self.current_session_mut() {
                        if session.request_subtab == RequestSubtab::Body {
                            if let Some(body) = &mut session.request.body {
                                if let Some(content) = &mut body.content {
                                    content.pop();
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            Action::Delete => {
                if self.focused_pane == Pane::UrlBar {
                    if let Some(session) = self.current_session_mut() {
                        if session.url_cursor_pos < session.request.url.len() {
                            session.request.url.remove(session.url_cursor_pos);
                        }
                    }
                }
            }
            Action::CursorLeft => {
                if self.focused_pane == Pane::UrlBar {
                    if let Some(session) = self.current_session_mut() {
                        if session.url_cursor_pos > 0 {
                            session.url_cursor_pos -= 1;
                        }
                    }
                }
            }
            Action::CursorRight => {
                if self.focused_pane == Pane::UrlBar {
                    if let Some(session) = self.current_session_mut() {
                        if session.url_cursor_pos < session.request.url.len() {
                            session.url_cursor_pos += 1;
                        }
                    }
                }
            }
            Action::SubmitInput => {
                if self.focused_pane == Pane::UrlBar {
                    let _ = self.update(Action::SendRequest);
                }
            }
            Action::ToggleRow => {
                if let Some(session) = self.current_session_mut() {
                    match session.request_subtab {
                        RequestSubtab::Query => {
                            if let Some(row) =
                                session.request.params.get_mut(session.table_selected_row)
                            {
                                row.enabled = !row.enabled;
                            }
                        }
                        RequestSubtab::Headers => {
                            if let Some(row) =
                                session.request.headers.get_mut(session.table_selected_row)
                            {
                                row.enabled = !row.enabled;
                            }
                        }
                        RequestSubtab::Path => {
                            if let Some(row) = session
                                .request
                                .path_params
                                .get_mut(session.table_selected_row)
                            {
                                row.enabled = !row.enabled;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Action::SelectNextRow => {
                if let Some(session) = self.current_session_mut() {
                    let max = match session.request_subtab {
                        RequestSubtab::Query => session.request.params.len(),
                        RequestSubtab::Headers => session.request.headers.len(),
                        RequestSubtab::Path => session.request.path_params.len(),
                        _ => 0,
                    };
                    if max > 0 && session.table_selected_row + 1 < max {
                        session.table_selected_row += 1;
                    }
                }
            }
            Action::SelectPrevRow => {
                if let Some(session) = self.current_session_mut() {
                    if session.table_selected_row > 0 {
                        session.table_selected_row -= 1;
                    }
                }
            }
            Action::AddTableRow => {
                if let Some(session) = self.current_session() {
                    let subtab = session.request_subtab;
                    if matches!(
                        subtab,
                        RequestSubtab::Query | RequestSubtab::Headers | RequestSubtab::Path
                    ) {
                        self.active_modal = Modal::EditTableRow {
                            target_subtab: subtab,
                            row_index: None,
                            key_input: String::new(),
                            value_input: String::new(),
                            active_field: 0,
                        };
                    }
                }
            }
            Action::EditTableRow => {
                if let Some(session) = self.current_session() {
                    let subtab = session.request_subtab;
                    match subtab {
                        RequestSubtab::Query => {
                            if let Some(row) =
                                session.request.params.get(session.table_selected_row)
                            {
                                self.active_modal = Modal::EditTableRow {
                                    target_subtab: subtab,
                                    row_index: Some(session.table_selected_row),
                                    key_input: row.name.clone(),
                                    value_input: row.value.clone(),
                                    active_field: 0,
                                };
                            }
                        }
                        RequestSubtab::Headers => {
                            if let Some(row) =
                                session.request.headers.get(session.table_selected_row)
                            {
                                self.active_modal = Modal::EditTableRow {
                                    target_subtab: subtab,
                                    row_index: Some(session.table_selected_row),
                                    key_input: row.name.clone(),
                                    value_input: row.value.clone(),
                                    active_field: 0,
                                };
                            }
                        }
                        RequestSubtab::Path => {
                            if let Some(row) =
                                session.request.path_params.get(session.table_selected_row)
                            {
                                self.active_modal = Modal::EditTableRow {
                                    target_subtab: subtab,
                                    row_index: Some(session.table_selected_row),
                                    key_input: row.name.clone(),
                                    value_input: row.value.clone(),
                                    active_field: 0,
                                };
                            }
                        }
                        RequestSubtab::Auth => {
                            match &session.request.auth {
                                AuthConfig::Bearer { token } => {
                                    self.active_modal = Modal::EditTableRow {
                                        target_subtab: subtab,
                                        row_index: Some(0),
                                        key_input: "Token".to_string(),
                                        value_input: token.clone(),
                                        active_field: 1,
                                    };
                                }
                                AuthConfig::Basic { username, password } => {
                                    self.active_modal = Modal::EditTableRow {
                                        target_subtab: subtab,
                                        row_index: Some(0),
                                        key_input: username.clone(),
                                        value_input: password.clone(),
                                        active_field: 0,
                                    };
                                }
                                AuthConfig::ApiKey { key, value, .. } => {
                                    self.active_modal = Modal::EditTableRow {
                                        target_subtab: subtab,
                                        row_index: Some(0),
                                        key_input: key.clone(),
                                        value_input: value.clone(),
                                        active_field: 0,
                                    };
                                }
                                AuthConfig::None => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
            Action::DeleteTableRow => {
                if let Some(session) = self.current_session_mut() {
                    match session.request_subtab {
                        RequestSubtab::Query => {
                            if session.table_selected_row < session.request.params.len() {
                                session.request.params.remove(session.table_selected_row);
                                if session.table_selected_row > 0
                                    && session.table_selected_row >= session.request.params.len()
                                {
                                    session.table_selected_row -= 1;
                                }
                            }
                        }
                        RequestSubtab::Headers => {
                            if session.table_selected_row < session.request.headers.len() {
                                session.request.headers.remove(session.table_selected_row);
                                if session.table_selected_row > 0
                                    && session.table_selected_row >= session.request.headers.len()
                                {
                                    session.table_selected_row -= 1;
                                }
                            }
                        }
                        RequestSubtab::Path => {
                            if session.table_selected_row < session.request.path_params.len() {
                                session.request.path_params.remove(session.table_selected_row);
                                if session.table_selected_row > 0
                                    && session.table_selected_row
                                        >= session.request.path_params.len()
                                {
                                    session.table_selected_row -= 1;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Action::SelectNextCollection => {
                let entries =
                    crate::ui::sidebar::build_sidebar_entries(&self.collections, &self.groups);
                let request_indices: Vec<usize> = entries
                    .iter()
                    .filter_map(|e| match e {
                        crate::ui::sidebar::SidebarEntry::RequestItem { collection_idx } => {
                            Some(*collection_idx)
                        }
                        _ => None,
                    })
                    .collect();
                if let Some(pos) = request_indices
                    .iter()
                    .position(|&idx| idx == self.selected_collection)
                {
                    if pos + 1 < request_indices.len() {
                        self.selected_collection = request_indices[pos + 1];
                    }
                } else if let Some(&first) = request_indices.first() {
                    self.selected_collection = first;
                }
                if let Some(line) = entries.iter().position(|e| {
                    matches!(e, crate::ui::sidebar::SidebarEntry::RequestItem { collection_idx } if *collection_idx == self.selected_collection)
                }) {
                    if line >= self.sidebar_scroll + 15 {
                        self.sidebar_scroll = line.saturating_sub(15) + 1;
                    }
                }
            }
            Action::SelectPrevCollection => {
                let entries =
                    crate::ui::sidebar::build_sidebar_entries(&self.collections, &self.groups);
                let request_indices: Vec<usize> = entries
                    .iter()
                    .filter_map(|e| match e {
                        crate::ui::sidebar::SidebarEntry::RequestItem { collection_idx } => {
                            Some(*collection_idx)
                        }
                        _ => None,
                    })
                    .collect();
                if let Some(pos) = request_indices
                    .iter()
                    .position(|&idx| idx == self.selected_collection)
                {
                    if pos > 0 {
                        self.selected_collection = request_indices[pos - 1];
                    }
                } else if let Some(&first) = request_indices.first() {
                    self.selected_collection = first;
                }
                if let Some(line) = entries.iter().position(|e| {
                    matches!(e, crate::ui::sidebar::SidebarEntry::RequestItem { collection_idx } if *collection_idx == self.selected_collection)
                }) {
                    if line < self.sidebar_scroll {
                        self.sidebar_scroll = line;
                    }
                }
            }
            Action::LoadSelectedCollection => {
                if let Some(item) = self.collections.get(self.selected_collection) {
                    if let Some(existing_idx) = self
                        .open_tabs
                        .iter()
                        .position(|t| t.file_path.as_deref() == Some(&item.path))
                    {
                        self.active_tab_index = existing_idx;
                    } else {
                        self.next_tab_id += 1;
                        let session = RequestTabSession::from_collection(self.next_tab_id, item);
                        self.open_tabs.push(session);
                        self.active_tab_index = self.open_tabs.len() - 1;
                    }
                    self.ensure_active_tab_visible();
                    self.focused_pane = Pane::UrlBar;
                    let _ = self.save_state();
                }
            }
            Action::ToggleSidebarView => {
                self.sidebar_view_mode = match self.sidebar_view_mode {
                    SidebarViewMode::Collections => SidebarViewMode::History,
                    SidebarViewMode::History => SidebarViewMode::Collections,
                };
                self.sidebar_scroll = 0;
            }
            Action::LoadSelectedHistory => {
                if let Some(hist) = self.history.get(self.selected_history_index) {
                    self.next_tab_id += 1;
                    let mut session = RequestTabSession::new(
                        self.next_tab_id,
                        format!("{} (History)", hist.request.name),
                        hist.request.clone(),
                        None,
                    );
                    session.active_response = Some(hist.response.clone());
                    self.open_tabs.push(session);
                    self.active_tab_index = self.open_tabs.len() - 1;
                    self.ensure_active_tab_visible();
                    self.focused_pane = Pane::ResponseViewer;
                    let _ = self.save_state();
                }
            }
            Action::ClearHistory => {
                self.history.clear();
                self.selected_history_index = 0;
                let _ = self.save_state();
            }
            Action::ScrollResponseUp => {
                if let Some(session) = self.current_session_mut() {
                    if session.response_scroll > 0 {
                        session.response_scroll -= 1;
                    }
                }
            }
            Action::ScrollResponseDown => {
                if let Some(session) = self.current_session_mut() {
                    session.response_scroll = session.response_scroll.saturating_add(1);
                }
            }
            Action::ScrollTabsLeft => {
                self.workspace_tabs_scroll = self.workspace_tabs_scroll.saturating_sub(8);
            }
            Action::ScrollTabsRight => {
                self.workspace_tabs_scroll = self.workspace_tabs_scroll.saturating_add(8);
            }
            Action::ScrollSidebarUp => {
                if self.sidebar_scroll > 0 {
                    self.sidebar_scroll -= 1;
                }
            }
            Action::ScrollSidebarDown => {
                self.sidebar_scroll = self.sidebar_scroll.saturating_add(1);
            }
            Action::OpenDeleteRequestModal => {
                if let Some(item) = self.collections.get(self.selected_collection) {
                    self.active_modal = Modal::ConfirmDeleteRequest {
                        name: item.name.clone(),
                        path: item.path.clone(),
                    };
                } else {
                    self.flash_message("No request selected to delete");
                }
            }
            Action::OpenDeleteGroupModal => {
                if self.groups.is_empty() {
                    self.flash_message("No collection groups exist to delete");
                } else if let Some(selected) = self.selected_group.clone() {
                    let group_path = self.config.collections_dir.join(&selected);
                    let count = self
                        .collections
                        .iter()
                        .filter(|c| c.group.as_deref() == Some(selected.as_str()))
                        .count();
                    self.active_modal = Modal::ConfirmDeleteGroup {
                        group_name: selected,
                        path: group_path,
                        req_count: count,
                    };
                } else if self.groups.len() == 1 {
                    let group_name = self.groups[0].clone();
                    let group_path = self.config.collections_dir.join(&group_name);
                    let count = self
                        .collections
                        .iter()
                        .filter(|c| c.group.as_deref() == Some(group_name.as_str()))
                        .count();
                    self.active_modal = Modal::ConfirmDeleteGroup {
                        group_name,
                        path: group_path,
                        req_count: count,
                    };
                } else {
                    self.active_modal = Modal::SelectGroupToDelete { selected_idx: 0 };
                }
            }
            Action::OpenClearHistoryModal => {
                if self.history.is_empty() {
                    self.flash_message("History is already empty");
                } else {
                    self.active_modal = Modal::ConfirmClearHistory;
                }
            }
            Action::DeleteSelectedHistory => {
                if !self.history.is_empty() && self.selected_history_index < self.history.len() {
                    let removed = self.history.remove(self.selected_history_index);
                    if self.selected_history_index >= self.history.len()
                        && self.selected_history_index > 0
                    {
                        self.selected_history_index -= 1;
                    }
                    let _ = self.save_state();
                    self.flash_message(format!("Deleted history entry #{}", removed.id));
                }
            }
            Action::OpenSearchModal => {
                let initial_q = self
                    .current_session()
                    .and_then(|s| s.search_query.clone())
                    .unwrap_or_default();
                self.active_modal = Modal::SearchResponse { query: initial_q };
            }
            Action::NextSearchMatch => {
                if let Some(session) = self.current_session_mut() {
                    session.next_search_match();
                }
            }
            Action::PrevSearchMatch => {
                if let Some(session) = self.current_session_mut() {
                    session.prev_search_match();
                }
            }
            Action::ClearSearch => {
                if let Some(session) = self.current_session_mut() {
                    session.update_search(None);
                }
            }
            Action::OpenCommandPalette => {
                self.active_modal = Modal::CommandPalette {
                    query: String::new(),
                    selected_idx: 0,
                };
            }
            Action::SaveRequest => {
                let default_dir = self.config.collections_dir.clone();
                let save_info = if let Some(session) = self.current_session_mut() {
                    let target = if let Some(path) = &session.file_path {
                        path.clone()
                    } else {
                        let sanitized_name = session
                            .request
                            .name
                            .to_lowercase()
                            .replace([' ', '/', '\\'], "_");
                        let file_name = format!("{}.posting.yaml", sanitized_name);
                        default_dir.join(file_name)
                    };
                    Some((target, session.request.clone()))
                } else {
                    None
                };

                if let Some((target_path, req)) = save_info {
                    let _ = req.save_to_file(&target_path);
                    if let Some(session) = self.current_session_mut() {
                        session.file_path = Some(target_path);
                    }
                }

                if let Ok(updated) = CollectionItem::scan_directory(&self.config.collections_dir) {
                    self.collections = updated;
                }
                self.flash_message("Saved request to collection");
                let _ = self.save_state();
            }
            Action::NextEnvironment => {
                if !self.environments.is_empty() {
                    self.active_env_index =
                        (self.active_env_index + 1) % self.environments.len();
                    let name = self.environments[self.active_env_index].name.clone();
                    self.flash_message(format!("Switched Environment: {}", name));
                }
            }
            Action::NextAuthType => {
                if let Some(session) = self.current_session_mut() {
                    session.request.auth = session.request.auth.next_type();
                }
            }
            Action::PrevAuthType => {
                if let Some(session) = self.current_session_mut() {
                    session.request.auth = session.request.auth.prev_type();
                }
            }
            Action::CopyAsCurl => {
                if let Some(session) = self.current_session() {
                    let resolved = if let Some(env) = self.active_environment() {
                        env.resolve_request(&session.request)
                    } else {
                        session.request.clone()
                    };
                    let curl_cmd = to_curl(&resolved);
                    if copy_to_clipboard(&curl_cmd).is_ok() {
                        self.flash_message("Copied cURL to clipboard!");
                    } else {
                        self.flash_message("Failed to access clipboard");
                    }
                }
            }
            Action::CopyResponseBody => {
                if let Some(session) = self.current_session() {
                    if let Some(res) = &session.active_response {
                        let formatted = res.formatted_body();
                        if copy_to_clipboard(&formatted).is_ok() {
                            self.flash_message("Copied Response Body to clipboard!");
                        } else {
                            self.flash_message("Failed to access clipboard");
                        }
                    }
                }
            }
            Action::OpenImportCurlModal | Action::OpenImportModal => {
                self.active_modal = Modal::ImportUniversal {
                    input: String::new(),
                };
            }
            Action::OpenExportModal => {
                self.active_modal = Modal::Export {
                    format_idx: 0,
                    path_input: String::new(),
                    active_field: 0,
                };
            }
            Action::OpenNewRequestModal => {
                let current_group = self
                    .collections
                    .get(self.selected_collection)
                    .and_then(|c| c.group.clone())
                    .unwrap_or_default();

                self.active_modal = Modal::NewRequest {
                    name_input: "New Request".to_string(),
                    selected_method: HttpMethod::Get,
                    group_input: current_group,
                    url_input: "https://httpbin.org/get".to_string(),
                    active_field: 0,
                };
            }
            Action::OpenNewGroupModal => {
                self.active_modal = Modal::NewGroup {
                    name_input: String::new(),
                };
            }
            Action::CloseModal => {
                self.active_modal = Modal::None;
            }
            Action::ModalNextField => match &mut self.active_modal {
                Modal::NewRequest { active_field, .. } => {
                    *active_field = (*active_field + 1) % 4;
                }
                Modal::EditTableRow { active_field, .. } => {
                    *active_field = (*active_field + 1) % 2;
                }
                Modal::Export { active_field, .. } => {
                    *active_field = (*active_field + 1) % 2;
                }
                _ => {}
            },
            Action::ModalPrevField => match &mut self.active_modal {
                Modal::NewRequest { active_field, .. } => {
                    *active_field = (*active_field + 3) % 4;
                }
                Modal::EditTableRow { active_field, .. } => {
                    *active_field = (*active_field + 1) % 2;
                }
                Modal::Export { active_field, .. } => {
                    *active_field = (*active_field + 1) % 2;
                }
                _ => {}
            },
            Action::SubmitModal => match &self.active_modal {
                Modal::NewRequest {
                    name_input,
                    selected_method,
                    group_input,
                    url_input,
                    ..
                } => {
                    let req_name = if name_input.trim().is_empty() {
                        "Request".to_string()
                    } else {
                        name_input.trim().to_string()
                    };
                    let mut new_req =
                        Request::new(req_name.clone(), *selected_method, url_input.trim());
                    new_req.headers = vec![KeyValuePair::new("User-Agent", "kurl/0.1.0")];

                    let sanitized = req_name.to_lowercase().replace([' ', '/', '\\'], "_");
                    let file_name = format!("{}.posting.yaml", sanitized);

                    let target_dir = if !group_input.trim().is_empty() {
                        CollectionItem::create_group(
                            &self.config.collections_dir,
                            group_input.trim(),
                        )?
                    } else {
                        self.config.collections_dir.clone()
                    };

                    let target_path = target_dir.join(file_name);
                    new_req.save_to_file(&target_path)?;

                    self.collections =
                        CollectionItem::scan_directory(&self.config.collections_dir)?;
                    self.groups =
                        CollectionItem::list_groups(&self.config.collections_dir).unwrap_or_default();

                    self.next_tab_id += 1;
                    let session = RequestTabSession::new(
                        self.next_tab_id,
                        req_name,
                        new_req,
                        Some(target_path),
                    );
                    self.open_tabs.push(session);
                    self.active_tab_index = self.open_tabs.len() - 1;

                    self.active_modal = Modal::None;
                    self.focused_pane = Pane::UrlBar;
                    let _ = self.save_state();
                }
                Modal::NewGroup { name_input } => {
                    if !name_input.trim().is_empty() {
                        let _ = CollectionItem::create_group(
                            &self.config.collections_dir,
                            name_input.trim(),
                        )?;
                        self.collections =
                            CollectionItem::scan_directory(&self.config.collections_dir)?;
                        self.groups =
                            CollectionItem::list_groups(&self.config.collections_dir).unwrap_or_default();
                    }
                    self.active_modal = Modal::None;
                }
                Modal::EditTableRow {
                    target_subtab,
                    row_index,
                    key_input,
                    value_input,
                    ..
                } => {
                    let subtab = *target_subtab;
                    let r_idx = *row_index;
                    let k = key_input.clone();
                    let v = value_input.clone();

                    if let Some(session) = self.current_session_mut() {
                        match subtab {
                            RequestSubtab::Query => {
                                if let Some(idx) = r_idx {
                                    if let Some(row) = session.request.params.get_mut(idx) {
                                        row.name = k;
                                        row.value = v;
                                    }
                                } else if !k.trim().is_empty() {
                                    session.request.params.push(KeyValuePair::new(k, v));
                                    session.table_selected_row = session.request.params.len() - 1;
                                }
                            }
                            RequestSubtab::Headers => {
                                if let Some(idx) = r_idx {
                                    if let Some(row) = session.request.headers.get_mut(idx) {
                                        row.name = k;
                                        row.value = v;
                                    }
                                } else if !k.trim().is_empty() {
                                    session.request.headers.push(KeyValuePair::new(k, v));
                                    session.table_selected_row = session.request.headers.len() - 1;
                                }
                            }
                            RequestSubtab::Path => {
                                if let Some(idx) = r_idx {
                                    if let Some(row) = session.request.path_params.get_mut(idx) {
                                        row.name = k;
                                        row.value = v;
                                    }
                                } else if !k.trim().is_empty() {
                                    session.request.path_params.push(KeyValuePair::new(k, v));
                                    session.table_selected_row =
                                        session.request.path_params.len() - 1;
                                }
                            }
                            RequestSubtab::Auth => match &mut session.request.auth {
                                AuthConfig::Bearer { token } => {
                                    *token = v;
                                }
                                AuthConfig::Basic { username, password } => {
                                    *username = k;
                                    *password = v;
                                }
                                AuthConfig::ApiKey { key, value, .. } => {
                                    *key = k;
                                    *value = v;
                                }
                                AuthConfig::None => {}
                            },
                            _ => {}
                        }
                    }
                    self.active_modal = Modal::None;
                }
                Modal::ImportUniversal { input } => {
                    let raw = input.trim();
                    if raw.is_empty() {
                        self.flash_message("Import input cannot be empty");
                        self.active_modal = Modal::None;
                        return Ok(());
                    }

                    match crate::formats::detect_import_payload(raw) {
                        Ok(payload) => match payload {
                            crate::formats::ImportPayload::Curl(curl_str) => {
                                if let Ok(req) = from_curl(curl_str.trim()) {
                                    self.next_tab_id += 1;
                                    let session = RequestTabSession::new(
                                        self.next_tab_id,
                                        req.name.clone(),
                                        req,
                                        None,
                                    );
                                    self.open_tabs.push(session);
                                    self.active_tab_index = self.open_tabs.len() - 1;
                                    self.focused_pane = Pane::UrlBar;
                                    self.flash_message("Imported cURL into new tab");
                                    let _ = self.save_state();
                                } else {
                                    self.flash_message("Failed to parse cURL command");
                                }
                            }
                            crate::formats::ImportPayload::PostmanCollection(json_str) => {
                                match crate::formats::import_postman_collection(&json_str) {
                                    Ok((coll_name, reqs)) => {
                                        let total = reqs.len();
                                        for (req, grp) in &reqs {
                                            let target_dir = if let Some(g) = grp {
                                                CollectionItem::create_group(&self.config.collections_dir, g).unwrap_or_else(|_| self.config.collections_dir.clone())
                                            } else {
                                                self.config.collections_dir.clone()
                                            };
                                            let sanitized = req.name.to_lowercase().replace([' ', '/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                                            let file_name = format!("{}.posting.yaml", sanitized);
                                            let target_path = target_dir.join(file_name);
                                            let _ = req.save_to_file(&target_path);
                                        }
                                        if let Ok(updated) = CollectionItem::scan_directory(&self.config.collections_dir) {
                                            self.collections = updated;
                                            self.groups = CollectionItem::list_groups(&self.config.collections_dir).unwrap_or_default();
                                        }
                                        if let Some(first) = self.collections.first() {
                                            self.next_tab_id += 1;
                                            let session = RequestTabSession::from_collection(self.next_tab_id, first);
                                            self.open_tabs.push(session);
                                            self.active_tab_index = self.open_tabs.len() - 1;
                                        }
                                        self.flash_message(format!("Imported {} requests from Postman '{}'", total, coll_name));
                                        let _ = self.save_state();
                                    }
                                    Err(e) => {
                                        self.flash_message(format!("Postman import error: {}", e));
                                    }
                                }
                            }
                            crate::formats::ImportPayload::PostmanEnvironment(json_str) => {
                                match crate::formats::import_postman_environment(&json_str) {
                                    Ok(env) => {
                                        let env_name = env.name.clone();
                                        let env_path = self.config.collections_dir.join("environments").join(format!("{}.env", env_name));
                                        let _ = env.save_to_env_file(&env_path);
                                        self.environments = Environment::scan_directory(&self.config.collections_dir);
                                        self.flash_message(format!("Imported Postman Environment '{}'", env_name));
                                    }
                                    Err(e) => {
                                        self.flash_message(format!("Environment import error: {}", e));
                                    }
                                }
                            }
                            crate::formats::ImportPayload::BrunoText(bru_str) => {
                                match crate::formats::parse_bru(&bru_str) {
                                    Ok(req) => {
                                        let sanitized = req.name.to_lowercase().replace([' ', '/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                                        let target_path = self.config.collections_dir.join(format!("{}.posting.yaml", sanitized));
                                        let _ = req.save_to_file(&target_path);
                                        if let Ok(updated) = CollectionItem::scan_directory(&self.config.collections_dir) {
                                            self.collections = updated;
                                        }
                                        self.next_tab_id += 1;
                                        let session = RequestTabSession::new(self.next_tab_id, req.name.clone(), req.clone(), Some(target_path));
                                        self.open_tabs.push(session);
                                        self.active_tab_index = self.open_tabs.len() - 1;
                                        self.flash_message(format!("Imported Bruno request '{}'", req.name));
                                        let _ = self.save_state();
                                    }
                                    Err(e) => {
                                        self.flash_message(format!("Bruno parse error: {}", e));
                                    }
                                }
                            }
                            crate::formats::ImportPayload::BrunoDirectory(dir_path) => {
                                match crate::formats::import_bruno_directory(&dir_path) {
                                    Ok(reqs) => {
                                        let total = reqs.len();
                                        for (req, grp) in &reqs {
                                            let target_dir = if let Some(g) = grp {
                                                CollectionItem::create_group(&self.config.collections_dir, g).unwrap_or_else(|_| self.config.collections_dir.clone())
                                            } else {
                                                self.config.collections_dir.clone()
                                            };
                                            let sanitized = req.name.to_lowercase().replace([' ', '/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                                            let file_name = format!("{}.posting.yaml", sanitized);
                                            let target_path = target_dir.join(file_name);
                                            let _ = req.save_to_file(&target_path);
                                        }
                                        if let Ok(updated) = CollectionItem::scan_directory(&self.config.collections_dir) {
                                            self.collections = updated;
                                            self.groups = CollectionItem::list_groups(&self.config.collections_dir).unwrap_or_default();
                                        }
                                        self.flash_message(format!("Imported {} requests from Bruno directory", total));
                                        let _ = self.save_state();
                                    }
                                    Err(e) => {
                                        self.flash_message(format!("Bruno dir import error: {}", e));
                                    }
                                }
                            }
                            crate::formats::ImportPayload::PostingYaml(yaml_str) => {
                                match Request::from_yaml_str(&yaml_str) {
                                    Ok(req) => {
                                        let sanitized = req.name.to_lowercase().replace([' ', '/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                                        let target_path = self.config.collections_dir.join(format!("{}.posting.yaml", sanitized));
                                        let _ = req.save_to_file(&target_path);
                                        if let Ok(updated) = CollectionItem::scan_directory(&self.config.collections_dir) {
                                            self.collections = updated;
                                        }
                                        self.next_tab_id += 1;
                                        let session = RequestTabSession::new(self.next_tab_id, req.name.clone(), req.clone(), Some(target_path));
                                        self.open_tabs.push(session);
                                        self.active_tab_index = self.open_tabs.len() - 1;
                                        self.flash_message(format!("Imported request '{}' from YAML", req.name));
                                        let _ = self.save_state();
                                    }
                                    Err(e) => {
                                        self.flash_message(format!("YAML parse error: {}", e));
                                    }
                                }
                            }
                            crate::formats::ImportPayload::File(path) => {
                                self.flash_message(format!("Imported file {:?}", path));
                            }
                        },
                        Err(e) => {
                            self.flash_message(format!("Auto-detect error: {}", e));
                        }
                    }
                    self.active_modal = Modal::None;
                }
                Modal::Export { format_idx, path_input, .. } => {
                    let dest = path_input.trim();
                    match *format_idx {
                        0 => {
                            match crate::formats::export_postman_collection("kurl_collection", &self.collections) {
                                Ok(json) => {
                                    if dest.is_empty() {
                                        let _ = copy_to_clipboard(&json);
                                        self.flash_message("Exported Postman Collection to clipboard");
                                    } else {
                                        let _ = std::fs::write(dest, json);
                                        self.flash_message(format!("Exported Postman Collection to '{}'", dest));
                                    }
                                }
                                Err(e) => {
                                    self.flash_message(format!("Postman export error: {}", e));
                                }
                            }
                        }
                        1 => {
                            if let Some(session) = self.current_session() {
                                let bru = crate::formats::to_bru(&session.request);
                                if dest.is_empty() {
                                    let _ = copy_to_clipboard(&bru);
                                    self.flash_message("Exported Bruno request to clipboard");
                                } else {
                                    let _ = std::fs::write(dest, bru);
                                    self.flash_message(format!("Exported Bruno request to '{}'", dest));
                                }
                            }
                        }
                        2 => {
                            if let Some(env) = self.active_environment() {
                                match crate::formats::export_postman_environment(env) {
                                    Ok(json) => {
                                        if dest.is_empty() {
                                            let _ = copy_to_clipboard(&json);
                                            self.flash_message("Exported Postman Environment to clipboard");
                                        } else {
                                            let _ = std::fs::write(dest, json);
                                            self.flash_message(format!("Exported Postman Environment to '{}'", dest));
                                        }
                                    }
                                    Err(e) => {
                                        self.flash_message(format!("Environment export error: {}", e));
                                    }
                                }
                            } else {
                                self.flash_message("No active environment to export");
                            }
                        }
                        3 => {
                            if let Some(session) = self.current_session() {
                                let curl = to_curl(&session.request);
                                if dest.is_empty() {
                                    let _ = copy_to_clipboard(&curl);
                                    self.flash_message("Exported cURL command to clipboard");
                                } else {
                                    let _ = std::fs::write(dest, curl);
                                    self.flash_message(format!("Exported cURL command to '{}'", dest));
                                }
                            }
                        }
                        4 => {
                            if let Some(session) = self.current_session() {
                                if let Ok(yaml) = session.request.to_yaml_str() {
                                    if dest.is_empty() {
                                        let _ = copy_to_clipboard(&yaml);
                                        self.flash_message("Exported YAML to clipboard");
                                    } else {
                                        let _ = std::fs::write(dest, yaml);
                                        self.flash_message(format!("Exported YAML to '{}'", dest));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    self.active_modal = Modal::None;
                }
                Modal::SearchResponse { .. } => {
                    self.active_modal = Modal::None;
                    self.focused_pane = Pane::ResponseViewer;
                }
                Modal::CommandPalette {
                    query,
                    selected_idx,
                } => {
                    let q_lower = query.trim().to_lowercase();
                    let matches: Vec<CollectionItem> = self
                        .collections
                        .iter()
                        .filter(|item| {
                            if q_lower.is_empty() {
                                return true;
                            }
                            item.name.to_lowercase().contains(&q_lower)
                                || item.request.method.as_str().to_lowercase().contains(&q_lower)
                                || item.request.url.to_lowercase().contains(&q_lower)
                                || item
                                    .group
                                    .as_deref()
                                    .unwrap_or("")
                                    .to_lowercase()
                                    .contains(&q_lower)
                        })
                        .cloned()
                        .collect();

                    if let Some(item) = matches.get(*selected_idx) {
                        if let Some(existing_idx) = self
                            .open_tabs
                            .iter()
                            .position(|t| t.file_path.as_deref() == Some(&item.path))
                        {
                            self.active_tab_index = existing_idx;
                        } else {
                            self.next_tab_id += 1;
                            let session =
                                RequestTabSession::from_collection(self.next_tab_id, item);
                            self.open_tabs.push(session);
                            self.active_tab_index = self.open_tabs.len() - 1;
                            let _ = self.save_state();
                        }
                        self.ensure_active_tab_visible();
                        self.focused_pane = Pane::UrlBar;
                    }
                    self.active_modal = Modal::None;
                }
                Modal::ConfirmDeleteRequest { name, path } => {
                    let _ = fs::remove_file(path);
                    for tab in &mut self.open_tabs {
                        if tab.file_path.as_ref() == Some(path) {
                            tab.file_path = None;
                        }
                    }
                    self.collections =
                        CollectionItem::scan_directory(&self.config.collections_dir)?;
                    if self.selected_collection >= self.collections.len()
                        && self.selected_collection > 0
                    {
                        self.selected_collection = self.collections.len() - 1;
                    }
                    let _ = self.save_state();
                    self.flash_message(format!("Deleted request '{}'", name));
                    self.active_modal = Modal::None;
                }
                Modal::ConfirmDeleteGroup {
                    group_name, path, ..
                } => {
                    let _ = fs::remove_dir_all(path);
                    for tab in &mut self.open_tabs {
                        if let Some(p) = &tab.file_path {
                            if p.starts_with(path) {
                                tab.file_path = None;
                            }
                        }
                    }
                    self.collections =
                        CollectionItem::scan_directory(&self.config.collections_dir)?;
                    self.groups =
                        CollectionItem::list_groups(&self.config.collections_dir).unwrap_or_default();
                    if self.selected_collection >= self.collections.len()
                        && self.selected_collection > 0
                    {
                        self.selected_collection = self.collections.len().saturating_sub(1);
                    }
                    self.selected_group = None;
                    let _ = self.save_state();
                    self.flash_message(format!("Deleted group '{}'", group_name));
                    self.active_modal = Modal::None;
                }
                Modal::SelectGroupToDelete { selected_idx } => {
                    if let Some(grp) = self.groups.get(*selected_idx).cloned() {
                        let group_path = self.config.collections_dir.join(&grp);
                        let req_count = self
                            .collections
                            .iter()
                            .filter(|c| c.group.as_deref() == Some(&grp))
                            .count();
                        self.active_modal = Modal::ConfirmDeleteGroup {
                            group_name: grp,
                            path: group_path,
                            req_count,
                        };
                    } else {
                        self.active_modal = Modal::None;
                    }
                }
                Modal::ConfirmClearHistory => {
                    self.history.clear();
                    self.selected_history_index = 0;
                    let _ = self.save_state();
                    self.flash_message("Cleared all call history");
                    self.active_modal = Modal::None;
                }
                Modal::None => {}
            },
            Action::ResizeSidebar(delta) => {
                self.sidebar_width = ((self.sidebar_width as i16) + delta).clamp(16, 80) as u16;
            }
            Action::ResizeVerticalSplit(delta) => {
                self.request_split_percent =
                    ((self.request_split_percent as i16) + delta).clamp(20, 80) as u16;
            }
            Action::StartDrag(target) => {
                self.active_drag = Some(target);
            }
            Action::EndDrag => {
                self.active_drag = None;
            }
        }
        Ok(())
    }
}
