use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: LogLevel,
    pub category: String,
    pub message: String,
    pub data: Option<String>,
}

#[derive(Debug, Default)]
pub struct Logger {
    log_path: std::path::PathBuf,
    logged_entries: HashMap<String, HashSet<String>>,
}

impl Logger {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let log_path = path.as_ref().to_path_buf();

        if !log_path.exists() {
            std::fs::write(&log_path, "")?;
        }

        Ok(Logger {
            log_path,
            logged_entries: HashMap::new(),
        })
    }

    fn has_been_logged(&self, category: &str, data: Option<&str>) -> bool {
        if let Some(data) = data {
            if let Some(entries) = self.logged_entries.get(category) {
                return entries.contains(data);
            }
        }
        false
    }

    fn mark_as_logged(&mut self, category: &str, data: Option<&str>) {
        if let Some(data) = data {
            self.logged_entries
                .entry(category.to_string())
                .or_insert_with(HashSet::new)
                .insert(data.to_string());
        }
    }

    pub fn log(
        &mut self,
        level: LogLevel,
        category: &str,
        message: &str,
        data: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.has_been_logged(category, data) {
            return Ok(());
        }

        let entry = LogEntry {
            level: level.clone(),
            category: category.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        if let Some(data_str) = &entry.data {
            writeln!(
                file,
                "[{}] {} - {}: {}",
                entry.level, entry.category, entry.message, data_str
            )?;
        } else {
            writeln!(
                file,
                "[{}] {} - {}",
                entry.level, entry.category, entry.message
            )?;
        }

        self.mark_as_logged(category, data);

        Ok(())
    }

    pub fn log_unparsed_sentence(
        &mut self,
        sentence: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.log(
            LogLevel::Warning,
            "unparsed_sentence",
            "Unable to parse sentence",
            Some(sentence),
        )
    }

    pub fn log_unknown_word(
        &mut self,
        word: &str,
        context: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.log(
            LogLevel::Warning,
            "unknown_word",
            &format!("In sentence: '{}'", context),
            Some(word),
        )
    }

    pub fn log_info(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.log(LogLevel::Info, "general", message, None)
    }

    pub fn log_error(
        &mut self,
        message: &str,
        details: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.log(LogLevel::Error, "error", message, details)
    }
}
