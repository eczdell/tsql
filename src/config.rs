use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_connection: String,
    pub connections: Vec<ConnectionConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectionConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub dbname: String,
    pub sslmode: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_connection: "nepal_police".to_string(),
            connections: vec![ConnectionConfig {
                name: "nepal_police".to_string(),
                host: "75.119.147.97".to_string(),
                port: 5432,
                user: "nepal_police".to_string(),
                password: Some("Postgres!2026Rotated".to_string()),
                dbname: "postgres".to_string(),
                sslmode: Some("disable".to_string()),
            }],
        }
    }
}

pub fn get_config_path() -> PathBuf {
    if let Some(mut path) = dirs::config_dir() {
        path.push("tsql");
        path.push("config.toml");
        path
    } else {
        PathBuf::from("tsql.toml")
    }
}

pub fn load_config() -> Config {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str(&content) {
                return cfg;
            }
        }
    } else {
        // Automatically ensure default config file exists
        let default_cfg = Config::default();
        let _ = save_config(&default_cfg);
    }
    Config::default()
}

pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    fs::write(path, content)?;
    Ok(())
}
