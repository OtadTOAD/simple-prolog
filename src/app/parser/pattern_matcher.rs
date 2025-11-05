use crate::app::{PrologApp, database::WordType};

#[derive(Debug, Clone)]
pub enum PatternToken {
    Literal(String),             // literal word match
    TypeMatch(Vec<WordType>),    // <Noun|Verb> matches any of the specified types
    Wildcard,                    // * matches any single word (not captured)
    Optional(Box<PatternToken>), // [token] matches 0 or 1 times
    Greedy(Box<PatternToken>), // token+ matches one or more times (captured and formatted as lowercase_with_underscores)
}

pub fn parse_pattern(pattern: &str) -> Vec<PatternToken> {
    let mut tokens = Vec::new();

    for element in pattern.split_whitespace() {
        if element.is_empty() {
            continue;
        }

        let (base_element, is_greedy) = if element.ends_with('+') && element.len() > 1 {
            (&element[..element.len() - 1], true)
        } else {
            (element, false)
        };

        let base_token = if base_element == "*" {
            Some(PatternToken::Wildcard)
        } else if base_element.starts_with('<') && base_element.ends_with('>') {
            let type_str = &base_element[1..base_element.len() - 1];
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
                Some(PatternToken::TypeMatch(types))
            } else {
                None
            }
        } else if base_element.starts_with('[') && base_element.ends_with(']') {
            let inner = &base_element[1..base_element.len() - 1];
            let inner_tokens = parse_pattern(inner);
            if let Some(inner_token) = inner_tokens.first() {
                Some(PatternToken::Optional(Box::new(inner_token.clone())))
            } else {
                None
            }
        } else {
            Some(PatternToken::Literal(base_element.to_string()))
        };

        if let Some(token) = base_token {
            if is_greedy {
                tokens.push(PatternToken::Greedy(Box::new(token)));
            } else {
                tokens.push(token);
            }
        }
    }

    tokens
}

pub fn matches_token(word: &str, token: &PatternToken, app: &PrologApp) -> bool {
    match token {
        PatternToken::Literal(literal) => word.eq_ignore_case(literal),
        PatternToken::TypeMatch(required_types) => {
            let Ok(read_database) = app.database.read() else {
                return false;
            };

            if let Some(entries) = read_database.get_word_entries(word) {
                entries
                    .iter()
                    .any(|entry| required_types.contains(&entry.word_type))
            } else {
                required_types.contains(&WordType::Noun)
            }
        }
        PatternToken::Wildcard => true,
        PatternToken::Optional(inner) => matches_token(word, inner, app),
        PatternToken::Greedy(inner) => matches_token(word, inner, app),
    }
}

pub fn try_match_pattern(
    words: &[String],
    pattern_tokens: &[PatternToken],
    app: &PrologApp,
) -> Option<Vec<String>> {
    fn backtrack(
        words: &[String],
        word_idx: usize,
        pattern_tokens: &[PatternToken],
        pattern_idx: usize,
        captures: &mut Vec<String>,
        app: &PrologApp,
    ) -> bool {
        if pattern_idx >= pattern_tokens.len() {
            return word_idx == words.len();
        }

        if word_idx >= words.len() {
            return pattern_tokens[pattern_idx..]
                .iter()
                .all(|t| matches!(t, PatternToken::Optional(_)));
        }

        match &pattern_tokens[pattern_idx] {
            PatternToken::Optional(inner) => {
                if matches_token(&words[word_idx], inner, app) {
                    if matches!(inner.as_ref(), PatternToken::TypeMatch(_)) {
                        captures.push(words[word_idx].clone());
                    }
                    if backtrack(
                        words,
                        word_idx + 1,
                        pattern_tokens,
                        pattern_idx + 1,
                        captures,
                        app,
                    ) {
                        return true;
                    }
                    if matches!(inner.as_ref(), PatternToken::TypeMatch(_)) {
                        captures.pop();
                    }
                }
                backtrack(
                    words,
                    word_idx,
                    pattern_tokens,
                    pattern_idx + 1,
                    captures,
                    app,
                )
            }
            PatternToken::Wildcard => backtrack(
                words,
                word_idx + 1,
                pattern_tokens,
                pattern_idx + 1,
                captures,
                app,
            ),
            PatternToken::Greedy(inner) => {
                let mut end_idx = word_idx;

                while end_idx < words.len() && matches_token(&words[end_idx], inner, app) {
                    end_idx += 1;
                }

                if end_idx == word_idx {
                    return false;
                }

                for try_end in (word_idx + 1..=end_idx).rev() {
                    let greedy_words: Vec<String> = words[word_idx..try_end].to_vec();
                    let formatted_capture = greedy_words.join(" ").to_lowercase().replace(' ', "_");

                    captures.push(formatted_capture);

                    if backtrack(
                        words,
                        try_end,
                        pattern_tokens,
                        pattern_idx + 1,
                        captures,
                        app,
                    ) {
                        return true;
                    }

                    captures.pop();
                }

                false
            }
            token => {
                if matches_token(&words[word_idx], token, app) {
                    if matches!(token, PatternToken::TypeMatch(_)) {
                        captures.push(words[word_idx].clone());
                    }
                    backtrack(
                        words,
                        word_idx + 1,
                        pattern_tokens,
                        pattern_idx + 1,
                        captures,
                        app,
                    )
                } else {
                    false
                }
            }
        }
    }

    let mut captures = Vec::new();
    if backtrack(words, 0, pattern_tokens, 0, &mut captures, app) {
        Some(captures)
    } else {
        None
    }
}

pub fn try_match_pattern_substring(
    words: &[String],
    pattern_tokens: &[PatternToken],
    app: &PrologApp,
) -> Option<(Vec<String>, usize)> {
    for start_idx in 0..words.len() {
        if let Some(captures) = try_match_pattern(&words[start_idx..], pattern_tokens, app) {
            return Some((captures, start_idx));
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub pattern_name: String,
    pub template: String,
    pub captures: Vec<String>,
    pub start_idx: usize,
    pub end_idx: usize,
}

fn try_match_at_position(
    words: &[String],
    start_idx: usize,
    pattern_tokens: &[PatternToken],
    pattern_name: &str,
    template: &str,
    app: &PrologApp,
) -> Option<PatternMatch> {
    fn backtrack_with_end(
        words: &[String],
        word_idx: usize,
        pattern_tokens: &[PatternToken],
        pattern_idx: usize,
        captures: &mut Vec<String>,
        app: &PrologApp,
    ) -> Option<usize> {
        if pattern_idx >= pattern_tokens.len() {
            return Some(word_idx);
        }

        if word_idx >= words.len() {
            if pattern_tokens[pattern_idx..]
                .iter()
                .all(|t| matches!(t, PatternToken::Optional(_)))
            {
                return Some(word_idx);
            }
            return None;
        }

        match &pattern_tokens[pattern_idx] {
            PatternToken::Optional(inner) => {
                if matches_token(&words[word_idx], inner, app) {
                    if matches!(inner.as_ref(), PatternToken::TypeMatch(_)) {
                        captures.push(words[word_idx].clone());
                    }
                    if let Some(end) = backtrack_with_end(
                        words,
                        word_idx + 1,
                        pattern_tokens,
                        pattern_idx + 1,
                        captures,
                        app,
                    ) {
                        return Some(end);
                    }
                    if matches!(inner.as_ref(), PatternToken::TypeMatch(_)) {
                        captures.pop();
                    }
                }
                backtrack_with_end(
                    words,
                    word_idx,
                    pattern_tokens,
                    pattern_idx + 1,
                    captures,
                    app,
                )
            }
            PatternToken::Wildcard => backtrack_with_end(
                words,
                word_idx + 1,
                pattern_tokens,
                pattern_idx + 1,
                captures,
                app,
            ),
            PatternToken::Greedy(inner) => {
                let mut matched_words = Vec::new();
                let mut end_idx = word_idx;

                while end_idx < words.len() && matches_token(&words[end_idx], inner, app) {
                    matched_words.push(words[end_idx].clone());
                    end_idx += 1;
                }

                if matched_words.is_empty() {
                    return None;
                }

                for try_end in (word_idx + 1..=end_idx).rev() {
                    let greedy_words: Vec<String> = words[word_idx..try_end].to_vec();
                    let formatted_capture = greedy_words.join(" ").to_lowercase().replace(' ', "_");

                    captures.push(formatted_capture);

                    if let Some(end) = backtrack_with_end(
                        words,
                        try_end,
                        pattern_tokens,
                        pattern_idx + 1,
                        captures,
                        app,
                    ) {
                        return Some(end);
                    }

                    captures.pop();
                }

                None
            }
            token => {
                if matches_token(&words[word_idx], token, app) {
                    if matches!(token, PatternToken::TypeMatch(_)) {
                        captures.push(words[word_idx].clone());
                    }
                    backtrack_with_end(
                        words,
                        word_idx + 1,
                        pattern_tokens,
                        pattern_idx + 1,
                        captures,
                        app,
                    )
                } else {
                    None
                }
            }
        }
    }

    let mut captures = Vec::new();
    if let Some(end_idx) = backtrack_with_end(
        &words[start_idx..],
        0,
        pattern_tokens,
        0,
        &mut captures,
        app,
    ) {
        Some(PatternMatch {
            pattern_name: pattern_name.to_string(),
            template: template.to_string(),
            captures,
            start_idx,
            end_idx: start_idx + end_idx,
        })
    } else {
        None
    }
}

pub fn find_all_pattern_matches(
    words: &[String],
    patterns: &[(String, String, Vec<PatternToken>)],
    app: &PrologApp,
) -> Vec<PatternMatch> {
    let is_conjunction = |word: &str| {
        matches!(
            word.to_lowercase().as_str(),
            "and" | "or" | "nor" | "but" | "yet" | ","
        )
    };

    let mut matches = Vec::new();
    let mut used_positions = vec![false; words.len()];

    loop {
        let mut best_match: Option<PatternMatch> = None;

        for (pattern_name, template, pattern_tokens) in patterns {
            for start_idx in 0..words.len() {
                if used_positions[start_idx] {
                    continue;
                }

                if is_conjunction(&words[start_idx]) {
                    continue;
                }

                if let Some(pattern_match) = try_match_at_position(
                    words,
                    start_idx,
                    pattern_tokens,
                    pattern_name,
                    template,
                    app,
                ) {
                    let overlap =
                        (pattern_match.start_idx..pattern_match.end_idx).any(|i| used_positions[i]);

                    if !overlap {
                        let match_len = pattern_match.end_idx - pattern_match.start_idx;
                        let best_len = best_match
                            .as_ref()
                            .map(|m| m.end_idx - m.start_idx)
                            .unwrap_or(0);

                        if match_len > best_len {
                            best_match = Some(pattern_match);
                        }
                    }
                }
            }
        }

        if let Some(m) = best_match {
            for i in m.start_idx..m.end_idx {
                used_positions[i] = true;
            }
            matches.push(m);
        } else {
            break;
        }
    }

    matches
}

pub fn apply_template(captures: &[String], template: &str) -> Vec<String> {
    let templates: Vec<&str> = template
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    let mut results = Vec::new();

    for tmpl in templates {
        let mut result = tmpl.to_string();

        for (i, word) in captures.iter().enumerate() {
            let placeholder = format!("${}", i + 1);
            result = result.replace(&placeholder, word);
        }

        results.push(result);
    }

    if results.is_empty() {
        results.push(template.to_string());
    }

    results
}

trait StrExt {
    fn eq_ignore_case(&self, other: &str) -> bool;
}

impl StrExt for str {
    fn eq_ignore_case(&self, other: &str) -> bool {
        self.eq_ignore_ascii_case(other)
    }
}
