use super::auth::AuthConfig;
use super::key_value::KeyValuePair;
use super::request::{Request, RequestBody};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub name: String,
    pub variables: HashMap<String, String>,
}

impl Environment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            variables: HashMap::new(),
        }
    }

    pub fn interpolate(&self, text: &str) -> String {
        let mut result = text.to_string();

        while result.contains("{{$uuid}}") {
            result = result.replacen("{{$uuid}}", &uuid::Uuid::new_v4().to_string(), 1);
        }

        let now = std::time::SystemTime::now();
        let epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let secs = epoch.as_secs().to_string();
        let millis = epoch.as_millis().to_string();

        result = result.replace("{{$timestamp}}", &secs);
        result = result.replace("{{$timestampMs}}", &millis);

        while result.contains("{{$randomInt}}") {
            let rnd = (epoch.as_nanos() % 90000 + 10000).to_string();
            result = result.replacen("{{$randomInt}}", &rnd, 1);
        }

        for (key, val) in &self.variables {
            let mustache_pattern = format!("{{{{{}}}}}", key);
            result = result.replace(&mustache_pattern, val);

            let bracket_pattern = format!("${{{}}}", key);
            result = result.replace(&bracket_pattern, val);

            let dollar_pattern = format!("${}", key);
            result = result.replace(&dollar_pattern, val);
        }

        if result.contains("{{") || result.contains('$') {
            for (env_key, env_val) in std::env::vars() {
                let mustache_pattern = format!("{{{{{}}}}}", env_key);
                result = result.replace(&mustache_pattern, &env_val);

                let bracket_pattern = format!("${{{}}}", env_key);
                result = result.replace(&bracket_pattern, &env_val);
            }
        }

        result
    }

    pub fn resolve_request(&self, req: &Request) -> Request {
        let mut resolved_url = self.interpolate(&req.url);

        for path_param in &req.path_params {
            if path_param.enabled && !path_param.name.trim().is_empty() {
                let token = path_param.name.trim();
                let token_clean = token.trim_start_matches(':');
                let token_colon = format!(":{}", token_clean);
                let resolved_val = self.interpolate(&path_param.value);

                resolved_url = resolved_url.replace(&token_colon, &resolved_val);
                resolved_url = resolved_url.replace(&format!("{{{}}}", token_clean), &resolved_val);
            }
        }

        let resolved_headers = req
            .headers
            .iter()
            .map(|h| {
                KeyValuePair::with_enabled(
                    self.interpolate(&h.name),
                    self.interpolate(&h.value),
                    h.enabled,
                )
            })
            .collect();

        let resolved_params = req
            .params
            .iter()
            .map(|p| {
                KeyValuePair::with_enabled(
                    self.interpolate(&p.name),
                    self.interpolate(&p.value),
                    p.enabled,
                )
            })
            .collect();

        let resolved_body = req.body.as_ref().map(|b| RequestBody {
            content: b.content.as_ref().map(|c| self.interpolate(c)),
        });

        let resolved_auth = match &req.auth {
            AuthConfig::None => AuthConfig::None,
            AuthConfig::Bearer { token } => AuthConfig::Bearer {
                token: self.interpolate(token),
            },
            AuthConfig::Basic { username, password } => AuthConfig::Basic {
                username: self.interpolate(username),
                password: self.interpolate(password),
            },
            AuthConfig::ApiKey {
                key,
                value,
                in_header,
            } => AuthConfig::ApiKey {
                key: self.interpolate(key),
                value: self.interpolate(value),
                in_header: *in_header,
            },
        };

        Request {
            name: req.name.clone(),
            description: req.description.clone(),
            method: req.method,
            url: resolved_url,
            auth: resolved_auth,
            headers: resolved_headers,
            params: resolved_params,
            path_params: req.path_params.clone(),
            body: resolved_body,
        }
    }

    pub fn from_env_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let name = path_ref
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "env".to_string());

        let content = fs::read_to_string(path_ref)?;
        let mut variables = HashMap::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                let clean_val = v.trim().trim_matches('"').trim_matches('\'');
                variables.insert(k.trim().to_string(), clean_val.to_string());
            }
        }

        Ok(Self { name, variables })
    }

    pub fn save_to_env_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut lines = Vec::new();
        let mut keys: Vec<_> = self.variables.keys().collect();
        keys.sort();
        for k in keys {
            if let Some(v) = self.variables.get(k) {
                lines.push(format!("{}={}", k, v));
            }
        }
        fs::write(p, lines.join("\n"))?;
        Ok(())
    }

    pub fn scan_directory<P: AsRef<Path>>(base_dir: P) -> Vec<Self> {
        let mut list = Vec::new();
        let path = base_dir.as_ref();

        let dot_env = Path::new(".env");
        if dot_env.exists() {
            if let Ok(env) = Self::from_env_file(dot_env) {
                list.push(env);
            }
        }

        let env_dir = path.join("environments");
        if env_dir.exists() {
            if let Ok(entries) = fs::read_dir(env_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Ok(env) = Self::from_env_file(&p) {
                            list.push(env);
                        }
                    }
                }
            }
        }

        if list.is_empty() {
            let mut default_env = Self::new("default");
            default_env
                .variables
                .insert("BASE_URL".to_string(), "https://httpbin.org".to_string());
            list.push(default_env);
        }

        list
    }
}
