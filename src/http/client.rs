use crate::config::Config;
use crate::models::{AuthConfig, KeyValuePair, Request, Response};
use anyhow::{Context, Result};
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;
use std::time::Instant;

#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new(config: &Config) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .timeout(config.http_timeout())
            .user_agent(&config.user_agent);

        if !config.follow_redirects {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        }

        let client = builder
            .build()
            .context("Failed to build reqwest HTTP client")?;

        Ok(Self { client })
    }

    pub async fn execute(&self, request: &Request) -> Result<Response> {
        let mut target_url = request.validate_url()?;

        {
            let mut query_pairs = target_url.query_pairs_mut();
            for param in &request.params {
                if param.enabled && !param.name.trim().is_empty() {
                    query_pairs.append_pair(param.name.trim(), &param.value);
                }
            }

            if let AuthConfig::ApiKey {
                key,
                value,
                in_header: false,
            } = &request.auth
            {
                if !key.trim().is_empty() {
                    query_pairs.append_pair(key.trim(), value);
                }
            }
        }

        let mut headers = HeaderMap::new();

        match &request.auth {
            AuthConfig::Bearer { token } => {
                if !token.trim().is_empty() {
                    let auth_val = format!("Bearer {}", token.trim());
                    if let Ok(val) = HeaderValue::from_str(&auth_val) {
                        headers.insert(reqwest::header::AUTHORIZATION, val);
                    }
                }
            }
            AuthConfig::Basic { username, password } => {
                let encoded = base64::engine::general_purpose::STANDARD
                    .encode(format!("{username}:{password}"));
                let auth_val = format!("Basic {}", encoded);
                if let Ok(val) = HeaderValue::from_str(&auth_val) {
                    headers.insert(reqwest::header::AUTHORIZATION, val);
                }
            }
            AuthConfig::ApiKey {
                key,
                value,
                in_header: true,
            } => {
                if !key.trim().is_empty() {
                    if let (Ok(h_name), Ok(h_val)) = (
                        HeaderName::from_str(key.trim()),
                        HeaderValue::from_str(value),
                    ) {
                        headers.insert(h_name, h_val);
                    }
                }
            }
            _ => {}
        }

        for header in &request.headers {
            if header.enabled && !header.name.trim().is_empty() {
                let name = HeaderName::from_str(header.name.trim())
                    .with_context(|| format!("Invalid header name: {}", header.name))?;
                let val = HeaderValue::from_str(&header.value)
                    .with_context(|| format!("Invalid header value for {}", header.name))?;
                headers.insert(name, val);
            }
        }

        let method = request.method.to_reqwest();
        let mut req_builder = self.client.request(method, target_url).headers(headers);

        if let Some(body) = &request.body {
            if let Some(content) = &body.content {
                if !content.is_empty() {
                    req_builder = req_builder.body(content.clone());
                }
            }
        }

        let start_time = Instant::now();
        let res = req_builder
            .send()
            .await
            .with_context(|| format!("Request failed for URL: {}", request.url))?;

        let duration_ms = start_time.elapsed().as_millis();
        let status = res.status();
        let status_code = status.as_u16();
        let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();

        let mut res_headers = Vec::new();
        let mut content_type = None;

        for (name, value) in res.headers() {
            let val_str = value.to_str().unwrap_or("").to_string();
            if name.as_str().eq_ignore_ascii_case("content-type") {
                content_type = Some(val_str.clone());
            }
            res_headers.push(KeyValuePair::new(name.as_str(), val_str));
        }

        let bytes = res
            .bytes()
            .await
            .context("Failed to read response body bytes")?;
        let body_size_bytes = bytes.len();
        let body_str = String::from_utf8_lossy(&bytes).to_string();

        Ok(Response {
            status_code,
            status_text,
            duration_ms,
            headers: res_headers,
            body: body_str,
            body_size_bytes,
            content_type,
        })
    }
}
