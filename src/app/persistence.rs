use crate::app::session::RequestTabSession;
use crate::events::SidebarViewMode;
use crate::models::HistoryItem;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSavedState {
    pub active_tab_index: usize,
    pub open_tabs: Vec<RequestTabSession>,
    pub sidebar_width: u16,
    pub request_split_percent: u16,
    pub active_env_index: usize,
    pub sidebar_view_mode: SidebarViewMode,
}

pub struct PersistenceManager {
    base_dir: PathBuf,
}

impl PersistenceManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        let local_fallback = PathBuf::from(".kurl");
        if local_fallback.is_dir() && !base_dir.join("state.json").exists() {
            let _ = fs::create_dir_all(&base_dir);
            if let Ok(state) = fs::read(local_fallback.join("state.json")) {
                let _ = fs::write(base_dir.join("state.json"), state);
            }
            if let Ok(hist) = fs::read(local_fallback.join("history.json")) {
                let _ = fs::write(base_dir.join("history.json"), hist);
            }
            let _ = fs::remove_dir_all(&local_fallback);
        }
        Self { base_dir }
    }

    pub fn state_file_path(&self) -> PathBuf {
        self.base_dir.join("state.json")
    }

    pub fn history_file_path(&self) -> PathBuf {
        self.base_dir.join("history.json")
    }

    pub fn save_state(&self, state: &AppSavedState) -> Result<()> {
        fs::create_dir_all(&self.base_dir).context("Failed to create persistence directory")?;
        let json = serde_json::to_string_pretty(state).context("Failed to serialize app state")?;
        fs::write(self.state_file_path(), json).context("Failed to write state.json")?;
        Ok(())
    }

    pub fn load_state(&self) -> Result<Option<AppSavedState>> {
        let path = self.state_file_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path).context("Failed to read state.json")?;
        let state = serde_json::from_str::<AppSavedState>(&content).context("Failed to parse state.json")?;
        Ok(Some(state))
    }

    pub fn save_history(&self, history: &[HistoryItem]) -> Result<()> {
        fs::create_dir_all(&self.base_dir).context("Failed to create persistence directory")?;
        let json = serde_json::to_string_pretty(history).context("Failed to serialize history")?;
        fs::write(self.history_file_path(), json).context("Failed to write history.json")?;
        Ok(())
    }

    pub fn load_history(&self) -> Result<Vec<HistoryItem>> {
        let path = self.history_file_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&path).context("Failed to read history.json")?;
        let history = serde_json::from_str::<Vec<HistoryItem>>(&content).context("Failed to parse history.json")?;
        Ok(history)
    }
}
