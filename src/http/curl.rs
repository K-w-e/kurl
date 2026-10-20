use crate::models::{AuthConfig, HttpMethod, KeyValuePair, Request, RequestBody};
use anyhow::{Context, Result};
use base64::Engine;

pub fn to_curl(req: &Request) -> String {
    let mut parts = Vec::new();
    parts.push("curl".to_string());

    if req.method != HttpMethod::Get {
        parts.push(format!("-X {}", req.method.as_str()));
    }

    let mut full_url = req.url.clone();
    let enabled_params: Vec<&KeyValuePair> = req
        .params
        .iter()
        .filter(|p| p.enabled && !p.name.trim().is_empty())
        .collect();

    if !enabled_params.is_empty() {
        let separator = if full_url.contains('?') { "&" } else { "?" };
        let query_str = enabled_params
            .iter()
            .map(|p| format!("{}={}", p.name.trim(), p.value.trim()))
            .collect::<Vec<_>>()
            .join("&");
        full_url = format!("{}{}{}", full_url, separator, query_str);
    }

    parts.push(format!("\"{}\"", full_url));

    match &req.auth {
        AuthConfig::Bearer { token } => {
            if !token.is_empty() {
                parts.push(format!("-H \"Authorization: Bearer {}\"", token));
            }
        }
        AuthConfig::Basic { username, password } => {
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
            parts.push(format!("-H \"Authorization: Basic {}\"", encoded));
        }
        AuthConfig::ApiKey {
            key,
            value,
            in_header,
        } => {
            if *in_header && !key.is_empty() {
                parts.push(format!("-H \"{}: {}\"", key, value));
            }
        }
        AuthConfig::None => {}
    }

    for header in &req.headers {
        if header.enabled && !header.name.trim().is_empty() {
            parts.push(format!("-H \"{}: {}\"", header.name.trim(), header.value));
        }
    }

    if let Some(body) = &req.body {
        if let Some(content) = &body.content {
            if !content.is_empty() {
                let escaped = content.replace('"', "\\\"");
                parts.push(format!("--data \"{}\"", escaped));
            }
        }
    }

    parts.join(" \\\n  ")
}

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new()
        .context("Failed to initialize system clipboard")?;
    clipboard
        .set_text(text)
        .context("Failed to write to system clipboard")?;
    Ok(())
}

pub fn from_curl(curl_cmd: &str) -> Result<Request> {
    let raw_tokens: Vec<&str> = curl_cmd.split_whitespace().collect();
    if raw_tokens.is_empty() {
        anyhow::bail!("Empty cURL command");
    }

    let mut method = HttpMethod::Get;
    let mut url = String::new();
    let mut headers = Vec::new();
    let mut body_content = None;

    let mut idx = 0;
    while idx < raw_tokens.len() {
        let token = raw_tokens[idx];
        match token {
            "-X" | "--request" => {
                if idx + 1 < raw_tokens.len() {
                    let m_str = raw_tokens[idx + 1].trim_matches('"').trim_matches('\'');
                    if let Ok(m) = m_str.parse::<HttpMethod>() {
                        method = m;
                    }
                    idx += 1;
                }
            }
            "-H" | "--header" => {
                if idx + 1 < raw_tokens.len() {
                    let mut full_header = raw_tokens[idx + 1].to_string();
                    idx += 1;
                    while !full_header.ends_with('"')
                        && !full_header.ends_with('\'')
                        && idx + 1 < raw_tokens.len()
                    {
                        idx += 1;
                        full_header.push(' ');
                        full_header.push_str(raw_tokens[idx]);
                    }
                    let clean = full_header.trim_matches('"').trim_matches('\'');
                    if let Some((k, v)) = clean.split_once(':') {
                        headers.push(KeyValuePair::new(k.trim(), v.trim()));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                if idx + 1 < raw_tokens.len() {
                    let mut full_body = raw_tokens[idx + 1].to_string();
                    idx += 1;
                    while !full_body.ends_with('"')
                        && !full_body.ends_with('\'')
                        && idx + 1 < raw_tokens.len()
                    {
                        idx += 1;
                        full_body.push(' ');
                        full_body.push_str(raw_tokens[idx]);
                    }
                    let clean = full_body.trim_matches('"').trim_matches('\'');
                    body_content = Some(clean.replace("\\\"", "\""));
                    if method == HttpMethod::Get {
                        method = HttpMethod::Post;
                    }
                }
            }
            other => {
                if other.starts_with("http://")
                    || other.starts_with("https://")
                    || other.starts_with("\"http://")
                    || other.starts_with("\"https://")
                    || other.starts_with("'http://")
                    || other.starts_with("'https://")
                {
                    url = other.trim_matches('"').trim_matches('\'').to_string();
                }
            }
        }
        idx += 1;
    }

    if url.is_empty() {
        anyhow::bail!("No valid HTTP/HTTPS URL found in cURL command");
    }

    let mut req = Request::new("Imported cURL", method, url);
    req.headers = headers;
    if let Some(content) = body_content {
        req.body = Some(RequestBody {
            content: Some(content),
        });
    }

    Ok(req)
}
