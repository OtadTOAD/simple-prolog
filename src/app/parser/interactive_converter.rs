use crate::app::{
    PrologApp,
    interactive_parser::{SentenceMatch, TokenHighlight, TokenType},
    parser::pattern_matcher::{PatternMatch, PatternToken},
};

pub fn create_interactive_match(
    words: &[String],
    pattern_match: &PatternMatch,
    pattern_tokens: &[PatternToken],
    app: &PrologApp,
) -> SentenceMatch {
    let mut highlights = Vec::new();
    let mut capture_index = 1;

    let mut word_to_capture = std::collections::HashMap::new();

    if let Some(captures_with_indices) = extract_captures_with_indices(words, pattern_tokens, app) {
        for (word_idx, word, token_type) in captures_with_indices {
            word_to_capture.insert(word_idx, capture_index);

            highlights.push(TokenHighlight {
                word: word.clone(),
                word_index: word_idx,
                capture_index,
                token_type,
                //is_editable: true,
            });

            capture_index += 1;
        }
    }

    let mut sentence_match = SentenceMatch {
        words: words.to_vec(),
        pattern_name: pattern_match.pattern_name.clone(),
        template: pattern_match.template.clone(),
        highlights,
        generated_output: String::new(),
    };

    sentence_match.regenerate_output();
    sentence_match
}

fn extract_captures_with_indices(
    words: &[String],
    pattern_tokens: &[PatternToken],
    app: &PrologApp,
) -> Option<Vec<(usize, String, TokenType)>> {
    fn backtrack(
        words: &[String],
        word_idx: usize,
        pattern_tokens: &[PatternToken],
        pattern_idx: usize,
        captures: &mut Vec<(usize, String, TokenType)>,
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
                    if let PatternToken::TypeMatch(types) = inner.as_ref() {
                        let token_type = word_type_to_token_type(&types[0]);
                        captures.push((word_idx, words[word_idx].clone(), token_type));
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
                    if let PatternToken::TypeMatch(_) = inner.as_ref() {
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
                let mut matched_words = Vec::new();
                let mut end_idx = word_idx;

                while end_idx < words.len() && matches_token(&words[end_idx], inner, app) {
                    matched_words.push(words[end_idx].clone());
                    end_idx += 1;
                }

                for try_end in (word_idx + 1..=end_idx).rev() {
                    let greedy_words = &words[word_idx..try_end];
                    captures.push((word_idx, greedy_words.join("_"), TokenType::Greedy));

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
                    if let PatternToken::TypeMatch(types) = token {
                        let token_type = word_type_to_token_type(&types[0]);
                        captures.push((word_idx, words[word_idx].clone(), token_type));
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

fn matches_token(word: &str, token: &PatternToken, app: &PrologApp) -> bool {
    use crate::app::database::WordType;

    match token {
        PatternToken::Literal(literal) => word.eq_ignore_ascii_case(literal),
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

fn word_type_to_token_type(word_type: &crate::app::database::WordType) -> TokenType {
    use crate::app::database::WordType;

    match word_type {
        WordType::Noun => TokenType::Noun,
        WordType::Verb => TokenType::Verb,
        WordType::Adjective => TokenType::Adjective,
        other => TokenType::Other(other.clone()),
    }
}
