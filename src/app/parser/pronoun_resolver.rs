/// Simple pronoun resolution using heuristics without gender
///
/// Heuristics:
/// - Singular pronouns (he, she, him, her, it) -> most recent singular noun (likely proper noun/unknown word)
/// - Plural pronouns (they, them) -> most recent plural noun (word ending in 's')
/// - Reflexive pronouns (himself, herself, themselves) -> subject of current sentence
/// - Possessive pronouns (his, her, their) -> possessive form of antecedent
use crate::app::database::{Database, WordType};
use std::sync::{Arc, RwLock};

/// Pronoun categories
#[derive(Debug, Clone, PartialEq)]
enum PronounType {
    SingularSubject, // he, she, it
    SingularObject,  // him, her, it
    PluralSubject,   // they
    PluralObject,    // them
    Possessive,      // his, her, their, its
    Reflexive,       // himself, herself, themselves, itself
}

#[derive(Debug, Clone)]
struct Entity {
    word: String,
    is_plural: bool,
    is_proper_noun: bool, // Likely a name (not in database)
}

pub struct PronounResolver {
    entities: Vec<Entity>,
    current_sentence_index: usize,
}

impl PronounResolver {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            current_sentence_index: 0,
        }
    }

    pub fn next_sentence(&mut self) {
        self.current_sentence_index += 1;
    }

    pub fn resolve_sentence(
        &mut self,
        words: &[String],
        database: &Arc<RwLock<Database>>,
    ) -> Vec<String> {
        let mut resolved = Vec::new();
        let mut subject_entity: Option<String> = None;

        for (_, word) in words.iter().enumerate() {
            let word_lower = word.to_lowercase();

            if let Some(pronoun_type) = self.identify_pronoun(&word_lower) {
                if let Some(antecedent) = self.resolve_pronoun(&pronoun_type, &subject_entity) {
                    resolved.push(antecedent);
                } else {
                    resolved.push(word.clone());
                }
            } else {
                resolved.push(word.clone());

                let is_plural = self.is_plural_form(&word_lower);
                let is_proper_noun = self.is_likely_proper_noun(&word_lower, database);

                if self.is_noun(&word_lower, database) || is_proper_noun {
                    let entity = Entity {
                        word: word.clone(),
                        is_plural,
                        is_proper_noun,
                    };

                    if subject_entity.is_none() {
                        subject_entity = Some(word.clone());
                    }

                    self.entities.push(entity);
                }
            }
        }

        resolved
    }

    fn identify_pronoun(&self, word: &str) -> Option<PronounType> {
        match word {
            // Singular subject pronouns
            "he" | "she" | "it" => Some(PronounType::SingularSubject),

            // Singular object pronouns
            "him" => Some(PronounType::SingularObject),

            // Plural subject pronouns
            "they" => Some(PronounType::PluralSubject),

            // Plural object pronouns
            "them" => Some(PronounType::PluralObject),

            // Possessive pronouns (note: "her" can be possessive or object, treat as possessive)
            "his" | "her" | "hers" | "their" | "theirs" | "its" => Some(PronounType::Possessive),

            // Reflexive pronouns
            "himself" | "herself" | "itself" | "themselves" => Some(PronounType::Reflexive),

            _ => None,
        }
    }

    fn resolve_pronoun(
        &self,
        pronoun_type: &PronounType,
        subject_entity: &Option<String>,
    ) -> Option<String> {
        match pronoun_type {
            PronounType::SingularSubject | PronounType::SingularObject => {
                self.find_most_recent_entity(false, true)
            }

            PronounType::PluralSubject | PronounType::PluralObject => {
                self.find_most_recent_entity(true, false)
            }

            PronounType::Reflexive => subject_entity.clone(),

            PronounType::Possessive => self
                .find_most_recent_entity(false, true)
                .or_else(|| self.find_most_recent_entity(true, false)),
        }
    }

    fn find_most_recent_entity(&self, is_plural: bool, prefer_proper_noun: bool) -> Option<String> {
        for entity in self.entities.iter().rev() {
            if entity.is_plural == is_plural {
                if prefer_proper_noun && entity.is_proper_noun {
                    return Some(entity.word.clone());
                }
                if !prefer_proper_noun {
                    return Some(entity.word.clone());
                }
            }
        }

        if prefer_proper_noun {
            for entity in self.entities.iter().rev() {
                if entity.is_plural == is_plural {
                    return Some(entity.word.clone());
                }
            }
        }

        None
    }

    fn is_plural_form(&self, word: &str) -> bool {
        if word.ends_with("ies") || word.ends_with("es") || word.ends_with('s') {
            !matches!(
                word,
                "was"
                    | "is"
                    | "this"
                    | "class"
                    | "grass"
                    | "glass"
                    | "pass"
                    | "mass"
                    | "boss"
                    | "moss"
                    | "loss"
                    | "cross"
                    | "toss"
                    | "dress"
                    | "stress"
                    | "guess"
                    | "less"
                    | "bless"
                    | "chess"
                    | "press"
                    | "express"
                    | "process"
                    | "success"
                    | "access"
                    | "address"
            )
        } else {
            false
        }
    }

    fn is_likely_proper_noun(&self, word: &str, database: &Arc<RwLock<Database>>) -> bool {
        if let Ok(db) = database.read() {
            db.get_word_entries(word).is_none()
        } else {
            false
        }
    }

    fn is_noun(&self, word: &str, database: &Arc<RwLock<Database>>) -> bool {
        if let Ok(db) = database.read() {
            if let Some(entries) = db.get_word_entries(word) {
                return entries
                    .iter()
                    .any(|e| matches!(e.word_type, WordType::Noun));
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plural_detection() {
        let resolver = PronounResolver::new();
        assert!(resolver.is_plural_form("books"));
        assert!(resolver.is_plural_form("people"));
        assert!(resolver.is_plural_form("children"));
        assert!(!resolver.is_plural_form("book"));
        assert!(!resolver.is_plural_form("was"));
    }

    #[test]
    fn test_pronoun_identification() {
        let resolver = PronounResolver::new();
        assert_eq!(
            resolver.identify_pronoun("he"),
            Some(PronounType::SingularSubject)
        );
        assert_eq!(
            resolver.identify_pronoun("they"),
            Some(PronounType::PluralSubject)
        );
        assert_eq!(resolver.identify_pronoun("book"), None);
    }
}
