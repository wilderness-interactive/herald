use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub google: GoogleConfig,
    pub ads: AdsGlobalConfig,
    pub account: Vec<AccountConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AdsGlobalConfig {
    pub developer_token: String,
    pub login_customer_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AccountConfig {
    pub name: String,
    pub customer_id: String,
    pub ga4_property_id: Option<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    FileNotFound(String),
    ParseError(String),
    WriteError(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => write!(f, "Config file not found: {path}"),
            ConfigError::ParseError(msg) => write!(f, "Failed to parse config: {msg}"),
            ConfigError::WriteError(msg) => write!(f, "Failed to write config: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn load_config(path: &str) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|_| ConfigError::FileNotFound(path.to_owned()))?;

    let config: Config = toml::from_str(&contents)
        .map_err(|e| ConfigError::ParseError(e.to_string()))?;

    Ok(config)
}

pub fn save_config(path: &str, config: &Config) -> Result<(), ConfigError> {
    let contents = toml::to_string_pretty(config)
        .map_err(|e| ConfigError::WriteError(e.to_string()))?;

    std::fs::write(path, contents)
        .map_err(|e| ConfigError::WriteError(e.to_string()))?;

    Ok(())
}
