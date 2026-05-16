use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Default)]
pub struct State {
    pub seen: HashSet<String>,
    pub last_opened: Option<DateTime<Utc>>,
}

impl State {
    pub fn load(path: &str) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                serde_json::from_str(&content).with_context(|| "Failed to parse state file")
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("Failed to read state file: {path}")),
        }
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content).with_context(|| format!("Failed to write state file: {path}"))
    }

    pub fn is_seen(&self, id: &str) -> bool {
        self.seen.contains(id)
    }

    pub fn mark_seen(&mut self, id: String) {
        self.seen.insert(id);
    }
}
