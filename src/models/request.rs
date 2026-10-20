use super::auth::AuthConfig;
use super::http_method::HttpMethod;
use super::key_value::KeyValuePair;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default, skip_serializing_if = "AuthConfig::is_none")]
    pub auth: AuthConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<KeyValuePair>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<KeyValuePair>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path_params: Vec<KeyValuePair>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<RequestBody>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            name: "New Request".to_string(),
            description: None,
            method: HttpMethod::Get,
            url: "https://httpbin.org/get".to_string(),
            auth: AuthConfig::None,
            headers: vec![KeyValuePair::new("User-Agent", "kurl/0.1.0")],
            params: Vec::new(),
            path_params: Vec::new(),
            body: None,
        }
    }
}

impl Request {
    pub fn new(name: impl Into<String>, method: HttpMethod, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            method,
            url: url.into(),
            auth: AuthConfig::None,
            headers: Vec::new(),
            params: Vec::new(),
            path_params: Vec::new(),
            body: None,
        }
    }

    pub fn validate_url(&self) -> Result<reqwest::Url> {
        let trimmed = self.url.trim();
        if trimmed.is_empty() {
            anyhow::bail!("URL cannot be empty");
        }
        let parsed = reqwest::Url::parse(trimmed)
            .with_context(|| format!("Invalid URL format: '{trimmed}'"))?;

        match parsed.scheme() {
            "http" | "https" => Ok(parsed),
            scheme => anyhow::bail!("Unsupported scheme '{scheme}'. Only http and https are allowed"),
        }
    }

    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).context("Failed to parse YAML request")
    }

    pub fn to_yaml_str(&self) -> Result<String> {
        serde_yaml::to_string(self).context("Failed to serialize request to YAML")
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read file: {:?}", path.as_ref()))?;
        Self::from_yaml_str(&content)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let yaml = self.to_yaml_str()?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }
        fs::write(path.as_ref(), yaml)
            .with_context(|| format!("Failed to write file: {:?}", path.as_ref()))
    }
}
