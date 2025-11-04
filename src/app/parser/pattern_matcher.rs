use crate::app::{PrologApp, database::WordType};

#[derive(Debug, Clone)]
pub enum PatternToken {
    Literal(String),
    TypeMatch(Vec<WordType>),
}

pub fn parse_pattern(pattern: &str) -> Vec<PatternToken> {
    let mut tokens = Vec::new();
    let mut chars = pattern.chars().peekable();
    let mut current = String::new();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            if !current.trim().is_empty() {
                tokens.push(PatternToken::Literal(current.trim().to_string()));
                current.clear();
            }

            let mut type_str = String::new();
            while let Some(&next_ch) = chars.peek() {
                if next_ch == '>' {
                    chars.next();
                    break;
                }
                type_str.push(chars.next().unwrap());
            }

            let types: Vec<WordType> = type_str
                .split('|')
                .filter_map(|s| match s.trim() {
                    "Noun" => Some(WordType::Noun),
                    "Verb" => Some(WordType::Verb),
                    "Adjective" => Some(WordType::Adjective),
                    "Adverb" => Some(WordType::Adverb),
                    "Pronoun" => Some(WordType::Pronoun),
                    "Preposition" => Some(WordType::Preposition),
                    "Conjunction" => Some(WordType::Conjunction),
                    "Interjection" => Some(WordType::Interjection),
                    "Determiner" => Some(WordType::Determiner),
                    _ => None,
                })
                .collect();

            if !types.is_empty() {
                tokens.push(PatternToken::TypeMatch(types));
            }
        } else if ch.is_whitespace() {
            if !current.trim().is_empty() {
                tokens.push(PatternToken::Literal(current.trim().to_string()));
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }

    if !current.trim().is_empty() {
        tokens.push(PatternToken::Literal(current.trim().to_string()));
    }

    tokens
}

pub fn matches_token(word: &str, token: &PatternToken, app: &PrologApp) -> bool {
    match token {
        PatternToken::Literal(literal) => word == literal,
        PatternToken::TypeMatch(required_types) => {
            let Ok(read_database) = app.database.read() else {
                return false;
            };

            if let Some(entries) = read_database.get_word_entries(word) {
                entries
                    .iter()
                    .any(|entry| required_types.contains(&entry.word_type))
            } else {
                false
            }
        }
    }
}

pub fn try_match_pattern(
    words: &[String],
    pattern_tokens: &[PatternToken],
    app: &PrologApp,
) -> bool {
    if words.len() != pattern_tokens.len() {
        return false;
    }

    words
        .iter()
        .zip(pattern_tokens.iter())
        .all(|(word, token)| matches_token(word, token, app))
}

pub fn apply_template(words: &[String], template: &str) -> String {
    let mut result = template.to_string();

    for (i, word) in words.iter().enumerate() {
        let placeholder = format!("${}", i + 1);
        result = result.replace(&placeholder, word);
    }

    result
}
