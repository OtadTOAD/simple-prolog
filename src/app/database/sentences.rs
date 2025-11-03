use serde::{Deserialize, Serialize};

use crate::app::database::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrologPattern {
    pub name: String,
    pub pattern: String,
    pub template: String,
    pub priority: i32,
    pub enabled: bool,
}

impl Database {
    pub fn get_sorted_patterns(&self) -> Vec<&PrologPattern> {
        let mut patterns: Vec<&PrologPattern> =
            self.patterns.iter().filter(|p| p.enabled).collect();

        patterns.sort_by(|a, b| b.priority.cmp(&a.priority));
        patterns
    }
}
