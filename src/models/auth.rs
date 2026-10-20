use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    #[default]
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        in_header: bool,
    },
}

impl AuthConfig {
    pub fn is_none(&self) -> bool {
        matches!(self, AuthConfig::None)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            AuthConfig::None => "None",
            AuthConfig::Bearer { .. } => "Bearer Token",
            AuthConfig::Basic { .. } => "Basic Auth",
            AuthConfig::ApiKey { .. } => "API Key",
        }
    }

    pub fn next_type(&self) -> Self {
        match self {
            AuthConfig::None => AuthConfig::Bearer {
                token: String::new(),
            },
            AuthConfig::Bearer { .. } => AuthConfig::Basic {
                username: String::new(),
                password: String::new(),
            },
            AuthConfig::Basic { .. } => AuthConfig::ApiKey {
                key: String::new(),
                value: String::new(),
                in_header: true,
            },
            AuthConfig::ApiKey { .. } => AuthConfig::None,
        }
    }

    pub fn prev_type(&self) -> Self {
        match self {
            AuthConfig::None => AuthConfig::ApiKey {
                key: String::new(),
                value: String::new(),
                in_header: true,
            },
            AuthConfig::Bearer { .. } => AuthConfig::None,
            AuthConfig::Basic { .. } => AuthConfig::Bearer {
                token: String::new(),
            },
            AuthConfig::ApiKey { .. } => AuthConfig::Basic {
                username: String::new(),
                password: String::new(),
            },
        }
    }
}
