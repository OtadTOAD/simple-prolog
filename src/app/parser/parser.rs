/*
    CHEAT SHEET:

    Determiners(Examples): a, an, the, this, that, these, those, my, your, his, her, its, our, their
    Noun(Examples): cat, dog, house, car, tree, book, idea, happiness
    Verb(Examples): run, jump, swim, read, write, think, be, have
    Adjective(Examples): big, small, red, quick, happy, sad, bright
    Adverb(Examples): quickly, slowly, happily, sadly, very, quite
    Preposition(Examples): in, on, at, by, with, about, against, between
    Conjunction(Examples): and, but, or, so, yet, for, nor
    Interjection(Examples): oh, wow, ouch, hey, alas, bravo
    Pronoun(Examples): I, you, he, she, it, we, they, me, him, her, us, them

    Sentence Structure Examples:
    Generic: noun_phrase + verb_phrase where =>
        noun_phrase = determiner + noun
        verb_phrase = verb + noun_phrase
    Example: "The(Determiner, could've been a) cat (noun_phrase) chased (verb_phrase) the(Determiner, could've been a) dog."

*/

use crate::app::PrologApp;

use super::{
    interactive_converter::create_interactive_match,
    pattern_matcher::{
        apply_template, find_all_pattern_matches, parse_pattern, try_match_pattern,
        try_match_pattern_substring,
    },
};

// Method for parsing input text chunk into sentences.
// This method assumes that input text will strictly follow grammatical rules.
// Specifically, sentences end with a period (.) followed by either a newline,
// carriage return, or a space followed by an uppercase letter.
// Each identified sentence is trimmed of leading and trailing whitespace
// before being added to the output vector.
pub fn parse_sentences(input: &String) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current_sentence = String::new();
    let chars: Vec<char> = input.chars().collect();

    for i in 0..chars.len() {
        let ch = chars[i];
        current_sentence.push(ch);

        if ch == '.' {
            let next_char = if i + 1 < chars.len() {
                Some(chars[i + 1])
            } else {
                None
            };

            let is_sentence_end = match next_char {
                None => true,                    // Is end of input
                Some('\n') | Some('\r') => true, // Newline or carriage return
                Some(' ') => {
                    // Space followed by uppercase letter
                    let mut j = i + 1;
                    while j < chars.len() && chars[j].is_whitespace() {
                        j += 1;
                    }
                    j < chars.len() && chars[j].is_uppercase()
                }
                _ => false,
            };

            if is_sentence_end {
                let trimmed = current_sentence.trim();
                if !trimmed.is_empty() {
                    sentences.push(trimmed.to_string().to_lowercase());
                }
                current_sentence.clear();
            }
        }
    }

    let trimmed = current_sentence.trim();
    if !trimmed.is_empty() {
        sentences.push(trimmed.to_string().to_lowercase());
    }

    sentences
}

pub fn parse_prolog(app: &mut PrologApp, sentence: &String) -> String {
    let words: Vec<String> = sentence
        .trim_end_matches('.')
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if words.is_empty() {
        return String::new();
    }

    let Ok(read_database) = app.database.read() else {
        return "// ERROR: Unable to read database\n".to_string();
    };

    let sorted_patterns = read_database.get_sorted_patterns();

    let patterns_with_tokens: Vec<(String, String, Vec<_>)> = sorted_patterns
        .iter()
        .map(|p| {
            (
                p.name.clone(),
                p.template.clone(),
                parse_pattern(&p.pattern),
            )
        })
        .collect();

    let has_conjunctions = words.iter().any(|w| {
        matches!(
            w.to_lowercase().as_str(),
            "and" | "or" | "nor" | "but" | "yet" | ","
        )
    });

    if !has_conjunctions {
        let matches = find_all_pattern_matches(&words, &patterns_with_tokens, &app);

        if !matches.is_empty() {
            for m in &matches {
                let pattern_tokens = parse_pattern(
                    &read_database
                        .patterns
                        .iter()
                        .find(|p| p.name == m.pattern_name)
                        .map(|p| &p.pattern)
                        .unwrap_or(&String::new()),
                );

                let interactive_match = create_interactive_match(
                    &words[m.start_idx..m.end_idx],
                    m,
                    &pattern_tokens,
                    app,
                );
                app.interactive_parser.matches.push(interactive_match);
            }

            let mut outputs = Vec::new();
            outputs.push(format!("// FROM: {}", sentence));

            for m in &matches {
                let prolog_outputs = apply_template(&m.captures, &m.template);
                outputs.push(format!(
                    "// PATTERN: {} (words {}-{})",
                    m.pattern_name, m.start_idx, m.end_idx
                ));
                outputs.extend(prolog_outputs);
            }

            return outputs.join("\n") + "\n";
        }
    }

    for &conj_idx in words
        .iter()
        .enumerate()
        .filter(|(_, w)| {
            matches!(
                w.to_lowercase().as_str(),
                "and" | "or" | "nor" | "but" | "yet" | ","
            )
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>()
        .iter()
    {
        let before_conj = &words[..conj_idx];
        let after_conj = &words[conj_idx + 1..];

        if before_conj.is_empty() || after_conj.is_empty() {
            continue;
        }

        let mut subject_end_idx = 0;
        for (i, word) in words.iter().enumerate() {
            if let Ok(read_database) = app.database.read() {
                if let Some(entries) = read_database.get_word_entries(word) {
                    if entries
                        .iter()
                        .any(|e| matches!(e.word_type, crate::app::database::WordType::Noun))
                    {
                        subject_end_idx = i + 1;
                        break;
                    }
                } else {
                    subject_end_idx = i + 1;
                    break;
                }
            }
        }

        if subject_end_idx > 0 && subject_end_idx <= conj_idx {
            let subject = &words[..subject_end_idx];
            let first_sentence = before_conj.to_vec();
            let mut second_sentence = subject.to_vec();
            second_sentence.extend_from_slice(after_conj);

            let mut first_match = None;
            let mut second_match = None;
            let mut first_pattern_name = String::new();
            let mut second_pattern_name = String::new();
            let mut first_pattern_tokens = Vec::new();
            let mut second_pattern_tokens = Vec::new();

            for pattern in sorted_patterns.iter() {
                let pattern_tokens = parse_pattern(&pattern.pattern);

                if first_match.is_none() {
                    if let Some(captures) =
                        try_match_pattern(&first_sentence, &pattern_tokens, &app)
                    {
                        first_match = Some((captures, pattern.template.clone()));
                        first_pattern_name = pattern.name.clone();
                        first_pattern_tokens = pattern_tokens.clone();
                    }
                }

                if second_match.is_none() {
                    if let Some(captures) =
                        try_match_pattern(&second_sentence, &pattern_tokens, &app)
                    {
                        second_match = Some((captures, pattern.template.clone()));
                        second_pattern_name = pattern.name.clone();
                        second_pattern_tokens = pattern_tokens.clone();
                    }
                }

                if first_match.is_some() && second_match.is_some() {
                    break;
                }
            }

            if let (
                Some((first_captures, first_template)),
                Some((second_captures, second_template)),
            ) = (first_match, second_match)
            {
                let first_pattern_match = super::pattern_matcher::PatternMatch {
                    pattern_name: first_pattern_name.clone(),
                    template: first_template.clone(),
                    captures: first_captures.clone(),
                    start_idx: 0,
                    end_idx: first_sentence.len(),
                };
                let first_interactive = create_interactive_match(
                    &first_sentence,
                    &first_pattern_match,
                    &first_pattern_tokens,
                    app,
                );
                app.interactive_parser.matches.push(first_interactive);

                let second_pattern_match = super::pattern_matcher::PatternMatch {
                    pattern_name: second_pattern_name.clone(),
                    template: second_template.clone(),
                    captures: second_captures.clone(),
                    start_idx: 0,
                    end_idx: second_sentence.len(),
                };
                let second_interactive = create_interactive_match(
                    &second_sentence,
                    &second_pattern_match,
                    &second_pattern_tokens,
                    app,
                );
                app.interactive_parser.matches.push(second_interactive);

                let mut outputs = Vec::new();
                outputs.push(format!("// FROM: {}", sentence));
                outputs.push(format!(
                    "// PATTERN: {} (conjunction expansion)",
                    first_pattern_name
                ));
                outputs.extend(apply_template(&first_captures, &first_template));
                outputs.push(format!("// PATTERN: {}", second_pattern_name));
                outputs.extend(apply_template(&second_captures, &second_template));
                return outputs.join("\n") + "\n";
            }
        }

        for split_point in (0..=before_conj.len()).rev() {
            let shared_prefix = &before_conj[..split_point];
            let first_part_suffix = &before_conj[split_point..];

            let mut first_sentence = shared_prefix.to_vec();
            first_sentence.extend_from_slice(first_part_suffix);

            let mut second_sentence = shared_prefix.to_vec();
            second_sentence.extend_from_slice(after_conj);

            for pattern in sorted_patterns.iter() {
                let pattern_tokens = parse_pattern(&pattern.pattern);

                if try_match_pattern(&first_sentence, &pattern_tokens, &app).is_some()
                    && try_match_pattern(&second_sentence, &pattern_tokens, &app).is_some()
                {
                    let mut outputs = Vec::new();
                    outputs.push(format!("// FROM: {}", sentence));
                    outputs.push(format!(
                        "// PATTERN: {} (with conjunction expansion)",
                        pattern.name
                    ));

                    if let Some(first_captures) =
                        try_match_pattern(&first_sentence, &pattern_tokens, &app)
                    {
                        let first_pattern_match = super::pattern_matcher::PatternMatch {
                            pattern_name: pattern.name.clone(),
                            template: pattern.template.clone(),
                            captures: first_captures.clone(),
                            start_idx: 0,
                            end_idx: first_sentence.len(),
                        };
                        let first_interactive = create_interactive_match(
                            &first_sentence,
                            &first_pattern_match,
                            &pattern_tokens,
                            app,
                        );
                        app.interactive_parser.matches.push(first_interactive);

                        outputs.extend(apply_template(&first_captures, &pattern.template));
                    }
                    if let Some(second_captures) =
                        try_match_pattern(&second_sentence, &pattern_tokens, &app)
                    {
                        let second_pattern_match = super::pattern_matcher::PatternMatch {
                            pattern_name: pattern.name.clone(),
                            template: pattern.template.clone(),
                            captures: second_captures.clone(),
                            start_idx: 0,
                            end_idx: second_sentence.len(),
                        };
                        let second_interactive = create_interactive_match(
                            &second_sentence,
                            &second_pattern_match,
                            &pattern_tokens,
                            app,
                        );
                        app.interactive_parser.matches.push(second_interactive);

                        outputs.extend(apply_template(&second_captures, &pattern.template));
                    }

                    return outputs.join("\n") + "\n";
                }
            }
        }
    }

    for pattern in sorted_patterns {
        let pattern_tokens = parse_pattern(&pattern.pattern);

        if let Some(captures) = try_match_pattern(&words, &pattern_tokens, &app) {
            let pattern_match = super::pattern_matcher::PatternMatch {
                pattern_name: pattern.name.clone(),
                template: pattern.template.clone(),
                captures: captures.clone(),
                start_idx: 0,
                end_idx: words.len(),
            };
            let interactive_match =
                create_interactive_match(&words, &pattern_match, &pattern_tokens, app);
            app.interactive_parser.matches.push(interactive_match);

            let prolog_outputs = apply_template(&captures, &pattern.template);
            let output = prolog_outputs.join("\n");
            return format!(
                "// FROM: {}\n// PATTERN: {}\n{}\n",
                sentence, pattern.name, output
            );
        }

        if let Some((captures, start_idx)) =
            try_match_pattern_substring(&words, &pattern_tokens, &app)
        {
            let match_len = captures
                .iter()
                .map(|c| c.split_whitespace().count())
                .sum::<usize>()
                .max(1);
            let pattern_match = super::pattern_matcher::PatternMatch {
                pattern_name: pattern.name.clone(),
                template: pattern.template.clone(),
                captures: captures.clone(),
                start_idx,
                end_idx: start_idx + match_len,
            };
            let interactive_match =
                create_interactive_match(&words[start_idx..], &pattern_match, &pattern_tokens, app);
            app.interactive_parser.matches.push(interactive_match);

            let prolog_outputs = apply_template(&captures, &pattern.template);
            let output = prolog_outputs.join("\n");
            return format!(
                "// FROM: {}\n// PATTERN: {} (substring match at word {})\n{}\n",
                sentence, pattern.name, start_idx, output
            );
        }
    }

    format!(
        "// FROM: {}\n// WARNING: No pattern matched\nprolog_fact('{}')\n",
        sentence,
        sentence.replace("'", "\\'")
    )
}

pub fn parse_input(app: &mut PrologApp, input: &String) -> String {
    app.interactive_parser.clear();
    let sentences = parse_sentences(input);

    let parsed_sentences: Vec<String> = sentences.iter().map(|s| parse_prolog(app, s)).collect();
    parsed_sentences.join("\n\n")
}
