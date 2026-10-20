use crate::models::{AuthConfig, CollectionItem, HttpMethod, KeyValuePair, Request, RequestBody};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn parse_bru(input: &str) -> Result<Request> {
    let blocks = extract_blocks(input);

    let mut req_name = "Bruno Request".to_string();
    if let Some(meta_content) = blocks.get("meta") {
        for line in meta_content.lines() {
            let trimmed = line.trim();
            if let Some((k, v)) = parse_key_value(trimmed) {
                if k == "name" {
                    req_name = v.to_string();
                }
            }
        }
    }

    let mut method = HttpMethod::Get;
    let mut url = String::new();

    let method_names = ["get", "post", "put", "delete", "patch", "head", "options"];
    for m in method_names {
        if let Some(content) = blocks.get(m) {
            method = match m {
                "post" => HttpMethod::Post,
                "put" => HttpMethod::Put,
                "delete" => HttpMethod::Delete,
                "patch" => HttpMethod::Patch,
                "head" => HttpMethod::Head,
                "options" => HttpMethod::Options,
                _ => HttpMethod::Get,
            };
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some((k, v)) = parse_key_value(trimmed) {
                    if k == "url" {
                        url = v.to_string();
                    }
                }
            }
            break;
        }
    }

    let mut headers = Vec::new();
    if let Some(h_content) = blocks.get("headers") {
        for line in h_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((k, v)) = parse_key_value(trimmed) {
                headers.push(KeyValuePair::new(k, v));
            }
        }
    }

    let mut params = Vec::new();
    if let Some(q_content) = blocks.get("params:query") {
        for line in q_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((k, v)) = parse_key_value(trimmed) {
                params.push(KeyValuePair::new(k, v));
            }
        }
    }

    let mut path_params = Vec::new();
    if let Some(p_content) = blocks.get("params:path") {
        for line in p_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((k, v)) = parse_key_value(trimmed) {
                path_params.push(KeyValuePair::new(k, v));
            }
        }
    }

    let auth = if let Some(b_auth) = blocks.get("auth:bearer") {
        let mut token = String::new();
        for line in b_auth.lines() {
            if let Some((k, v)) = parse_key_value(line.trim()) {
                if k == "token" {
                    token = v.to_string();
                }
            }
        }
        AuthConfig::Bearer { token }
    } else if let Some(basic_auth) = blocks.get("auth:basic") {
        let mut username = String::new();
        let mut password = String::new();
        for line in basic_auth.lines() {
            if let Some((k, v)) = parse_key_value(line.trim()) {
                if k == "username" {
                    username = v.to_string();
                } else if k == "password" {
                    password = v.to_string();
                }
            }
        }
        AuthConfig::Basic { username, password }
    } else if let Some(api_auth) = blocks.get("auth:apikey") {
        let mut key = String::new();
        let mut value = String::new();
        let mut placement = "header".to_string();
        for line in api_auth.lines() {
            if let Some((k, v)) = parse_key_value(line.trim()) {
                if k == "key" {
                    key = v.to_string();
                } else if k == "value" {
                    value = v.to_string();
                } else if k == "placement" {
                    placement = v.to_string();
                }
            }
        }
        let in_header = placement.to_lowercase() != "query";
        AuthConfig::ApiKey {
            key,
            value,
            in_header,
        }
    } else {
        AuthConfig::None
    };

    let body = if let Some(j_body) = blocks.get("body:json") {
        let trimmed = j_body.trim();
        if !trimmed.is_empty() {
            Some(RequestBody {
                content: Some(trimmed.to_string()),
            })
        } else {
            None
        }
    } else if let Some(t_body) = blocks.get("body:text") {
        let trimmed = t_body.trim();
        if !trimmed.is_empty() {
            Some(RequestBody {
                content: Some(trimmed.to_string()),
            })
        } else {
            None
        }
    } else {
        None
    };

    Ok(Request {
        name: req_name,
        description: None,
        method,
        url,
        auth,
        headers,
        params,
        path_params,
        body,
    })
}

fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.splitn(2, ':');
    let k = parts.next()?.trim();
    let v = parts.next()?.trim();
    if !k.is_empty() {
        Some((k, v))
    } else {
        None
    }
}

fn extract_blocks(input: &str) -> HashMap<String, String> {
    let mut blocks = HashMap::new();
    let mut current_block_name: Option<String> = None;
    let mut current_block_content = String::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if current_block_name.is_none() {
            if trimmed.ends_with('{') {
                let name = trimmed.trim_end_matches('{').trim().to_lowercase();
                if !name.is_empty() {
                    current_block_name = Some(name);
                    current_block_content.clear();
                }
            }
        } else if trimmed == "}" {
            if let Some(name) = current_block_name.take() {
                blocks.insert(name, current_block_content.clone());
            }
        } else {
            current_block_content.push_str(line);
            current_block_content.push('\n');
        }
    }

    blocks
}

pub fn to_bru(req: &Request) -> String {
    let mut out = String::new();

    out.push_str("meta {\n");
    out.push_str(&format!("  name: {}\n", req.name));
    out.push_str("  type: http\n");
    out.push_str("  seq: 1\n");
    out.push_str("}\n\n");

    let method_str = req.method.as_str().to_lowercase();
    let body_type = if req.body.is_some() { "json" } else { "none" };
    let auth_type = match req.auth {
        AuthConfig::Bearer { .. } => "bearer",
        AuthConfig::Basic { .. } => "basic",
        AuthConfig::ApiKey { .. } => "apikey",
        AuthConfig::None => "none",
    };

    out.push_str(&format!("{} {{\n", method_str));
    out.push_str(&format!("  url: {}\n", req.url));
    out.push_str(&format!("  body: {}\n", body_type));
    out.push_str(&format!("  auth: {}\n", auth_type));
    out.push_str("}\n\n");

    if !req.headers.is_empty() {
        out.push_str("headers {\n");
        for h in &req.headers {
            if h.enabled {
                out.push_str(&format!("  {}: {}\n", h.name, h.value));
            }
        }
        out.push_str("}\n\n");
    }

    if !req.params.is_empty() {
        out.push_str("params:query {\n");
        for q in &req.params {
            if q.enabled {
                out.push_str(&format!("  {}: {}\n", q.name, q.value));
            }
        }
        out.push_str("}\n\n");
    }

    if !req.path_params.is_empty() {
        out.push_str("params:path {\n");
        for p in &req.path_params {
            if p.enabled {
                out.push_str(&format!("  {}: {}\n", p.name, p.value));
            }
        }
        out.push_str("}\n\n");
    }

    match &req.auth {
        AuthConfig::Bearer { token } => {
            out.push_str("auth:bearer {\n");
            out.push_str(&format!("  token: {}\n", token));
            out.push_str("}\n\n");
        }
        AuthConfig::Basic { username, password } => {
            out.push_str("auth:basic {\n");
            out.push_str(&format!("  username: {}\n", username));
            out.push_str(&format!("  password: {}\n", password));
            out.push_str("}\n\n");
        }
        AuthConfig::ApiKey { key, value, in_header } => {
            let placement = if *in_header { "header" } else { "query" };
            out.push_str("auth:apikey {\n");
            out.push_str(&format!("  key: {}\n", key));
            out.push_str(&format!("  value: {}\n", value));
            out.push_str(&format!("  placement: {}\n", placement));
            out.push_str("}\n\n");
        }
        AuthConfig::None => {}
    }

    if let Some(body) = &req.body {
        if let Some(content) = &body.content {
            out.push_str("body:json {\n");
            for line in content.lines() {
                out.push_str(&format!("  {}\n", line));
            }
            out.push_str("}\n");
        }
    }

    out
}

pub fn import_bruno_directory<P: AsRef<Path>>(dir: P) -> Result<Vec<(Request, Option<String>)>> {
    let mut out = Vec::new();
    let base = dir.as_ref();
    if !base.is_dir() {
        anyhow::bail!("Path is not a directory: {:?}", base);
    }

    scan_bruno_dir_recursive(base, base, &mut out)?;
    Ok(out)
}

fn scan_bruno_dir_recursive(
    base: &Path,
    current: &Path,
    out: &mut Vec<(Request, Option<String>)>,
) -> Result<()> {
    let entries = fs::read_dir(current)
        .with_context(|| format!("Failed to read directory: {:?}", current))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "bru" {
                    let content = fs::read_to_string(&path)
                        .with_context(|| format!("Failed to read bru file: {:?}", path))?;
                    if let Ok(req) = parse_bru(&content) {
                        let group = if current != base {
                            current
                                .strip_prefix(base)
                                .ok()
                                .map(|p| p.to_string_lossy().replace('\\', "/"))
                        } else {
                            None
                        };
                        out.push((req, group));
                    }
                }
            }
        } else if path.is_dir() {
            let dirname = path.file_name().unwrap_or_default().to_string_lossy();
            if !dirname.starts_with('.') && dirname != "node_modules" {
                let _ = scan_bruno_dir_recursive(base, &path, out);
            }
        }
    }

    Ok(())
}

pub fn export_bruno_directory<P: AsRef<Path>>(
    target_dir: P,
    items: &[CollectionItem],
) -> Result<()> {
    let base = target_dir.as_ref();
    fs::create_dir_all(base).context("Failed to create Bruno export directory")?;

    for item in items {
        let dir = if let Some(grp) = &item.group {
            let sanitized = grp.replace(['\\', ':', '*', '?', '"', '<', '>', '|'], "_");
            base.join(sanitized)
        } else {
            base.to_path_buf()
        };

        fs::create_dir_all(&dir)?;
        let sanitized_name = item
            .name
            .to_lowercase()
            .replace([' ', '/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let file_path = dir.join(format!("{}.bru", sanitized_name));
        let bru_str = to_bru(&item.request);
        fs::write(&file_path, bru_str)
            .with_context(|| format!("Failed to write Bruno file: {:?}", file_path))?;
    }

    Ok(())
}
