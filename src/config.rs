use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub filter: FilterConfig,
}

#[derive(Deserialize)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub api_hash: String,
    pub session_file: String,
    pub phone: Option<String>,
}

#[derive(Deserialize)]
pub struct FilterConfig {
    #[serde(default = "default_min_reactions")]
    pub min_negative_reactions: i32,
    #[serde(default = "default_negative_emojis")]
    pub negative_emojis: Vec<String>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            min_negative_reactions: default_min_reactions(),
            negative_emojis: default_negative_emojis(),
        }
    }
}

fn default_min_reactions() -> i32 {
    10
}

fn default_negative_emojis() -> Vec<String> {
    vec!["🤡".into(), "💩".into()]
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let path = expand_tilde(path);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {path}"))?;
        let mut config: Self =
            serde_json::from_str(&content).with_context(|| "Failed to parse config file")?;
        config.telegram.session_file = expand_tilde(&config.telegram.session_file);
        Ok(config)
    }
}
