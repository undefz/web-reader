use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub rss: Vec<String>,
    #[serde(default)]
    pub hacker_news: HackerNewsConfig,
    #[serde(default = "default_state_file")]
    pub state_file: String,
    #[serde(default)]
    pub filter: FilterConfig,
    #[serde(default = "default_cooldown_minutes")]
    pub cooldown_minutes: u64,
}

fn default_state_file() -> String {
    "~/.config/web.state.json".into()
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

#[derive(Deserialize)]
pub struct HackerNewsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_hn_limit")]
    pub limit: usize,
}

impl Default for HackerNewsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            limit: default_hn_limit(),
        }
    }
}

fn default_cooldown_minutes() -> u64 {
    0
}

fn default_hn_limit() -> usize {
    30
}

fn default_min_reactions() -> i32 {
    10
}

fn default_negative_emojis() -> Vec<String> {
    vec!["🤡".into(), "💩".into()]
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{rest}");
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
        config.state_file = expand_tilde(&config.state_file);
        Ok(config)
    }
}
