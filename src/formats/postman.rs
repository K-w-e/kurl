use crate::models::{AuthConfig, CollectionItem, Environment, HttpMethod, KeyValuePair, Request, RequestBody};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanCollection {
    pub info: Option<PostmanInfo>,
    #[serde(default)]
    pub item: Vec<PostmanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanInfo {
    pub name: String,
    pub description: Option<String>,
    pub schema: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanItem {
    pub name: String,
    pub request: Option<PostmanRequest>,
    #[serde(default)]
    pub item: Vec<PostmanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanRequest {
    pub method: Option<String>,
    pub url: Option<PostmanUrlValue>,
    pub description: Option<String>,
    #[serde(default)]
    pub header: Vec<PostmanHeader>,
    pub body: Option<PostmanRequestBody>,
    pub auth: Option<PostmanAuth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PostmanUrlValue {
    String(String),
    Object(PostmanUrlObject),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanUrlObject {
    pub raw: Option<String>,
    #[serde(default)]
    pub query: Vec<PostmanQueryParam>,
    #[serde(default)]
    pub variable: Vec<PostmanVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanQueryParam {
    pub key: Option<String>,
    pub value: Option<String>,
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanVariable {
    pub key: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanHeader {
    pub key: String,
    pub value: String,
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanRequestBody {
    pub mode: Option<String>,
    pub raw: Option<String>,
    pub urlencoded: Option<Vec<PostmanQueryParam>>,
    pub formdata: Option<Vec<PostmanQueryParam>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanAuth {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub bearer: Option<Vec<PostmanAuthParam>>,
    pub basic: Option<Vec<PostmanAuthParam>>,
    pub apikey: Option<Vec<PostmanAuthParam>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanAuthParam {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanEnvironmentJson {
    pub name: String,
    #[serde(default)]
    pub values: Vec<PostmanEnvValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanEnvValue {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

pub fn import_postman_collection(json_str: &str) -> Result<(String, Vec<(Request, Option<String>)>)> {
    let collection: PostmanCollection =
        serde_json::from_str(json_str).context("Failed to parse Postman collection JSON")?;

    let coll_name = collection
        .info
        .as_ref()
        .map(|i| i.name.clone())
        .unwrap_or_else(|| "Imported Postman Collection".to_string());

    let mut out = Vec::new();
    traverse_postman_items(&collection.item, None, &mut out);

    Ok((coll_name, out))
}

fn traverse_postman_items(
    items: &[PostmanItem],
    current_group: Option<&str>,
    out: &mut Vec<(Request, Option<String>)>,
) {
    for item in items {
        if let Some(postman_req) = &item.request {
            let req = convert_postman_request(&item.name, postman_req);
            out.push((req, current_group.map(|s| s.to_string())));
        }

        if !item.item.is_empty() {
            let next_group = match current_group {
                Some(parent) => format!("{}/{}", parent, item.name),
                None => item.name.clone(),
            };
            traverse_postman_items(&item.item, Some(&next_group), out);
        }
    }
}

fn convert_postman_request(name: &str, p_req: &PostmanRequest) -> Request {
    let method = match p_req.method.as_deref().unwrap_or("GET").to_uppercase().as_str() {
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "DELETE" => HttpMethod::Delete,
        "PATCH" => HttpMethod::Patch,
        "HEAD" => HttpMethod::Head,
        "OPTIONS" => HttpMethod::Options,
        _ => HttpMethod::Get,
    };

    let mut url_str = String::new();
    let mut params = Vec::new();
    let mut path_params = Vec::new();

    if let Some(url_val) = &p_req.url {
        match url_val {
            PostmanUrlValue::String(s) => {
                url_str = s.clone();
            }
            PostmanUrlValue::Object(obj) => {
                if let Some(raw) = &obj.raw {
                    url_str = raw.clone();
                }
                for q in &obj.query {
                    if let Some(k) = &q.key {
                        let enabled = !q.disabled.unwrap_or(false);
                        let v = q.value.clone().unwrap_or_default();
                        params.push(KeyValuePair {
                            name: k.clone(),
                            value: v,
                            enabled,
                        });
                    }
                }
                for v in &obj.variable {
                    if let Some(k) = &v.key {
                        let val = v.value.clone().unwrap_or_default();
                        path_params.push(KeyValuePair::new(k, val));
                    }
                }
            }
        }
    }

    let mut headers = Vec::new();
    for h in &p_req.header {
        let enabled = !h.disabled.unwrap_or(false);
        headers.push(KeyValuePair {
            name: h.key.clone(),
            value: h.value.clone(),
            enabled,
        });
    }

    let auth = if let Some(p_auth) = &p_req.auth {
        match p_auth.auth_type.to_lowercase().as_str() {
            "bearer" => {
                let token = p_auth
                    .bearer
                    .as_ref()
                    .and_then(|params| params.iter().find(|p| p.key == "token"))
                    .map(|p| p.value.clone())
                    .unwrap_or_default();
                AuthConfig::Bearer { token }
            }
            "basic" => {
                let username = p_auth
                    .basic
                    .as_ref()
                    .and_then(|params| params.iter().find(|p| p.key == "username"))
                    .map(|p| p.value.clone())
                    .unwrap_or_default();
                let password = p_auth
                    .basic
                    .as_ref()
                    .and_then(|params| params.iter().find(|p| p.key == "password"))
                    .map(|p| p.value.clone())
                    .unwrap_or_default();
                AuthConfig::Basic { username, password }
            }
            "apikey" => {
                let key = p_auth
                    .apikey
                    .as_ref()
                    .and_then(|params| params.iter().find(|p| p.key == "key"))
                    .map(|p| p.value.clone())
                    .unwrap_or_default();
                let value = p_auth
                    .apikey
                    .as_ref()
                    .and_then(|params| params.iter().find(|p| p.key == "value"))
                    .map(|p| p.value.clone())
                    .unwrap_or_default();
                let placement = p_auth
                    .apikey
                    .as_ref()
                    .and_then(|params| params.iter().find(|p| p.key == "in"))
                    .map(|p| p.value.clone())
                    .unwrap_or_else(|| "header".to_string());
                let in_header = placement.to_lowercase() != "query";
                AuthConfig::ApiKey {
                    key,
                    value,
                    in_header,
                }
            }
            _ => AuthConfig::None,
        }
    } else {
        AuthConfig::None
    };

    let body = if let Some(p_body) = &p_req.body {
        let content = if let Some(raw) = &p_body.raw {
            Some(raw.clone())
        } else if let Some(urlencoded) = &p_body.urlencoded {
            let encoded: Vec<String> = urlencoded
                .iter()
                .filter(|q| !q.disabled.unwrap_or(false))
                .filter_map(|q| {
                    q.key.as_ref().map(|k| {
                        let v = q.value.as_deref().unwrap_or("");
                        format!("{}={}", k, v)
                    })
                })
                .collect();
            Some(encoded.join("&"))
        } else {
            None
        };
        content.map(|c| RequestBody { content: Some(c) })
    } else {
        None
    };

    Request {
        name: name.to_string(),
        description: p_req.description.clone(),
        method,
        url: url_str,
        auth,
        headers,
        params,
        path_params,
        body,
    }
}

pub fn export_postman_collection(collection_name: &str, items: &[CollectionItem]) -> Result<String> {
    let mut root_items = Vec::new();
    let mut group_map: HashMap<String, Vec<PostmanItem>> = HashMap::new();

    for item in items {
        let p_req = convert_to_postman_request(&item.request);
        let p_item = PostmanItem {
            name: item.name.clone(),
            request: Some(p_req),
            item: Vec::new(),
        };

        if let Some(grp) = &item.group {
            group_map.entry(grp.clone()).or_default().push(p_item);
        } else {
            root_items.push(p_item);
        }
    }

    let mut sorted_groups: Vec<_> = group_map.into_iter().collect();
    sorted_groups.sort_by(|a, b| a.0.cmp(&b.0));

    for (group_name, group_items) in sorted_groups {
        let folder = PostmanItem {
            name: group_name,
            request: None,
            item: group_items,
        };
        root_items.push(folder);
    }

    let collection = PostmanCollection {
        info: Some(PostmanInfo {
            name: collection_name.to_string(),
            description: Some("Exported from kurl".to_string()),
            schema: Some("https://schema.getpostman.com/json/collection/v2.1.0/collection.json".to_string()),
        }),
        item: root_items,
    };

    serde_json::to_string_pretty(&collection).context("Failed to serialize Postman collection")
}

fn convert_to_postman_request(req: &Request) -> PostmanRequest {
    let headers = req
        .headers
        .iter()
        .map(|h| PostmanHeader {
            key: h.name.clone(),
            value: h.value.clone(),
            disabled: if h.enabled { None } else { Some(true) },
        })
        .collect();

    let query = req
        .params
        .iter()
        .map(|q| PostmanQueryParam {
            key: Some(q.name.clone()),
            value: Some(q.value.clone()),
            disabled: if q.enabled { None } else { Some(true) },
        })
        .collect();

    let variable = req
        .path_params
        .iter()
        .map(|p| PostmanVariable {
            key: Some(p.name.clone()),
            value: Some(p.value.clone()),
        })
        .collect();

    let url_obj = PostmanUrlObject {
        raw: Some(req.url.clone()),
        query,
        variable,
    };

    let body = req.body.as_ref().and_then(|b| b.content.as_ref()).map(|c| PostmanRequestBody {
        mode: Some("raw".to_string()),
        raw: Some(c.clone()),
        urlencoded: None,
        formdata: None,
    });

    let auth = match &req.auth {
        AuthConfig::Bearer { token } => Some(PostmanAuth {
            auth_type: "bearer".to_string(),
            bearer: Some(vec![PostmanAuthParam {
                key: "token".to_string(),
                value: token.clone(),
            }]),
            basic: None,
            apikey: None,
        }),
        AuthConfig::Basic { username, password } => Some(PostmanAuth {
            auth_type: "basic".to_string(),
            bearer: None,
            basic: Some(vec![
                PostmanAuthParam {
                    key: "username".to_string(),
                    value: username.clone(),
                },
                PostmanAuthParam {
                    key: "password".to_string(),
                    value: password.clone(),
                },
            ]),
            apikey: None,
        }),
        AuthConfig::ApiKey { key, value, in_header } => Some(PostmanAuth {
            auth_type: "apikey".to_string(),
            bearer: None,
            basic: None,
            apikey: Some(vec![
                PostmanAuthParam {
                    key: "key".to_string(),
                    value: key.clone(),
                },
                PostmanAuthParam {
                    key: "value".to_string(),
                    value: value.clone(),
                },
                PostmanAuthParam {
                    key: "in".to_string(),
                    value: if *in_header { "header".to_string() } else { "query".to_string() },
                },
            ]),
        }),
        AuthConfig::None => None,
    };

    PostmanRequest {
        method: Some(req.method.as_str().to_string()),
        url: Some(PostmanUrlValue::Object(url_obj)),
        description: req.description.clone(),
        header: headers,
        body,
        auth,
    }
}

pub fn import_postman_environment(json_str: &str) -> Result<Environment> {
    let env_json: PostmanEnvironmentJson =
        serde_json::from_str(json_str).context("Failed to parse Postman environment JSON")?;

    let mut env = Environment::new(env_json.name);
    for v in env_json.values {
        if v.enabled {
            env.variables.insert(v.key, v.value);
        }
    }

    Ok(env)
}

pub fn export_postman_environment(env: &Environment) -> Result<String> {
    let mut values = Vec::new();
    let mut sorted_keys: Vec<_> = env.variables.keys().collect();
    sorted_keys.sort();

    for k in sorted_keys {
        if let Some(v) = env.variables.get(k) {
            values.push(PostmanEnvValue {
                key: k.clone(),
                value: v.clone(),
                enabled: true,
            });
        }
    }

    let env_json = PostmanEnvironmentJson {
        name: env.name.clone(),
        values,
    };

    serde_json::to_string_pretty(&env_json).context("Failed to serialize Postman environment")
}
