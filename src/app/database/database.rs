use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

use crate::app::database::{sentences::PrologPattern, words::WordEntry};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Database {
    #[serde(default)]
    pub words: Vec<WordEntry>,
    #[serde(default)]
    pub patterns: Vec<PrologPattern>,

    #[serde(skip)]
    pub form_index: HashMap<String, String>,
    #[serde(skip)]
    pub form_value: HashMap<String, WordEntry>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.as_ref();

        if path.exists() {
            let extension = path.extension().and_then(|s| s.to_str());

            let mut db: Database = if extension == Some("bin") {
                // Load binary format (fast)
                let data = std::fs::read(path)?;
                bincode::deserialize(&data)?
            } else {
                // Load JSON format (human-readable, slower)
                let data = std::fs::read_to_string(path)?;
                serde_json::from_str(&data)?
            };

            db.rebuild_index();
            Ok(db)
        } else {
            let db = Database::default();
            db.save(path)?;
            Ok(db)
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref();
        let extension = path.extension().and_then(|s| s.to_str());

        if extension == Some("bin") {
            let data = bincode::serialize(self)?;
            std::fs::write(path, data)?;
        } else {
            let data = serde_json::to_string_pretty(&self)?;
            std::fs::write(path, data)?;
        }

        Ok(())
    }

    /// Convert JSON database to binary format
    pub fn convert_to_binary(
        json_path: &str,
        bin_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let db = Self::new(json_path)?;
        db.save(bin_path)?;
        Ok(())
    }

    pub fn rebuild_index(&mut self) {
        self.form_index.clear();
        self.form_value.clear();

        for entry in &self.words {
            self.form_index
                .insert(entry.lemma.clone(), entry.lemma.clone());
            self.form_value.insert(entry.lemma.clone(), entry.clone());

            for form in &entry.forms {
                self.form_index.insert(form.clone(), entry.lemma.clone());
            }
        }
    }
}
