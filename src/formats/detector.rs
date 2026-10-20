use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportPayload {
    PostmanCollection(String),
    PostmanEnvironment(String),
    BrunoText(String),
    BrunoDirectory(PathBuf),
    Curl(String),
    PostingYaml(String),
    File(PathBuf),
}

pub fn detect_import_payload(raw_input: &str) -> Result<ImportPayload> {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Input cannot be empty");
    }

    let path = Path::new(trimmed);
    if path.exists() {
        if path.is_dir() {
            return Ok(ImportPayload::BrunoDirectory(path.to_path_buf()));
        } else if path.is_file() {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read file: {:?}", path))?;
            return detect_content(&content, Some(path));
        }
    }

    detect_content(trimmed, None)
}

fn detect_content(content: &str, original_path: Option<&Path>) -> Result<ImportPayload> {
    let trimmed = content.trim();

    if trimmed.starts_with("curl ") || trimmed.starts_with("curl.exe ") {
        return Ok(ImportPayload::Curl(trimmed.to_string()));
    }

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if (trimmed.contains("\"item\"") || trimmed.contains("\"schema\""))
            && trimmed.contains("\"info\"")
        {
            return Ok(ImportPayload::PostmanCollection(trimmed.to_string()));
        }

        if trimmed.contains("\"values\"") && !trimmed.contains("\"item\"") {
            return Ok(ImportPayload::PostmanEnvironment(trimmed.to_string()));
        }

        if trimmed.contains("\"item\"") {
            return Ok(ImportPayload::PostmanCollection(trimmed.to_string()));
        }
    }

    if trimmed.contains("meta {")
        || (trimmed.contains("{\n")
            && (trimmed.contains("get {")
                || trimmed.contains("post {")
                || trimmed.contains("put {")
                || trimmed.contains("delete {")
                || trimmed.contains("params:query {")
                || trimmed.contains("headers {")))
    {
        return Ok(ImportPayload::BrunoText(trimmed.to_string()));
    }

    if (trimmed.contains("name:") || trimmed.contains("method:")) && trimmed.contains("url:") {
        return Ok(ImportPayload::PostingYaml(trimmed.to_string()));
    }

    if let Some(path) = original_path {
        return Ok(ImportPayload::File(path.to_path_buf()));
    }

    anyhow::bail!("Unrecognized import format. Supported formats: Postman Collection JSON, Postman Environment JSON, Bruno .bru, cURL command, or Posting YAML.")
}
