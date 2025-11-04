use std::fmt;

use serde::{Deserialize, Serialize};

use crate::app::database::Database;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WordType {
    Noun,
    Verb,
    Adjective,
    Adverb,
    Pronoun,
    Preposition,
    Conjunction,
    Interjection,
    Determiner,
}

impl fmt::Display for WordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WordType::Noun => write!(f, "Noun"),
            WordType::Verb => write!(f, "Verb"),
            WordType::Adjective => write!(f, "Adjective"),
            WordType::Adverb => write!(f, "Adverb"),
            WordType::Pronoun => write!(f, "Pronoun"),
            WordType::Preposition => write!(f, "Preposition"),
            WordType::Conjunction => write!(f, "Conjunction"),
            WordType::Interjection => write!(f, "Interjection"),
            WordType::Determiner => write!(f, "Determiner"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordEntry {
    pub lemma: String,
    pub word_type: WordType,
    pub forms: Vec<String>,
}

impl Database {
    pub fn get_word_entries(&self, word: &str) -> Option<&Vec<WordEntry>> {
        let key = self.form_index.get(word)?;
        self.form_value.get(key)
    }
}
