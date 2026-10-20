use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub tick_rate_ms: u64,
    pub http_timeout_seconds: u64,
    pub collections_dir: PathBuf,
    pub persistence_dir: PathBuf,
    pub user_agent: String,
    pub follow_redirects: bool,
}

impl Config {
    pub fn global_dir() -> PathBuf {
        dirs::data_dir()
            .or_else(dirs::config_dir)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join(".kurl"))
                    .unwrap_or_else(|| PathBuf::from("."))
            })
            .join("kurl")
    }

    pub fn resolve_collections_dir(explicit: Option<PathBuf>) -> PathBuf {
        if let Some(path) = explicit {
            return path;
        }

        let local_collections = Path::new("./collections");
        if local_collections.is_dir() {
            return local_collections.to_path_buf();
        }

        let local_posting = Path::new("./.posting");
        if local_posting.is_dir() {
            return local_posting.to_path_buf();
        }

        Self::global_dir().join("collections")
    }

    pub fn new(explicit_collections: Option<PathBuf>) -> Self {
        let global = Self::global_dir();
        let collections_dir = Self::resolve_collections_dir(explicit_collections);

        Self {
            tick_rate_ms: 16,
            http_timeout_seconds: 30,
            collections_dir,
            persistence_dir: global,
            user_agent: "kurl/0.1.0".to_string(),
            follow_redirects: true,
        }
    }

    pub fn tick_duration(&self) -> Duration {
        Duration::from_millis(self.tick_rate_ms)
    }

    pub fn http_timeout(&self) -> Duration {
        Duration::from_secs(self.http_timeout_seconds)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(None)
    }
}
