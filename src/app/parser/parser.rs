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

use crate::app::{PrologApp, logger::LogLevel};

use super::pattern_matcher::{
    apply_template, find_all_pattern_matches, parse_pattern, try_match_pattern,
    try_match_pattern_substring,
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

    // Prepare patterns in the format needed for find_all_pattern_matches
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

    println!("DEBUG: Trying to find all pattern matches in: {:?}", words);

    // Check if there are conjunctions in the sentence
    let has_conjunctions = words.iter().any(|w| {
        matches!(
            w.to_lowercase().as_str(),
            "and" | "or" | "nor" | "but" | "yet" | ","
        )
    });

    // If there are conjunctions, skip find_all_pattern_matches and use conjunction expansion instead
    if !has_conjunctions {
        // Find all non-overlapping pattern matches
        let matches = find_all_pattern_matches(&words, &patterns_with_tokens, &app);

        if !matches.is_empty() {
            // Generate output for all matches
            let mut outputs = Vec::new();
            outputs.push(format!("// FROM: {}", sentence));

            for m in &matches {
                println!(
                    "DEBUG: MATCH! Pattern '{}' at [{}, {}): {:?}",
                    m.pattern_name, m.start_idx, m.end_idx, m.captures
                );

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

    // If no matches found, try the old single-pattern approach as fallback
    // But first, try conjunction expansion with ALL patterns

    // Try conjunction expansion across all patterns
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

        // Find the subject (first noun)
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
                    // Unknown word, assume it's a proper noun
                    subject_end_idx = i + 1;
                    break;
                }
            }
        }

        // Try subject sharing: "Plato wrote books and put words..."
        // -> "Plato wrote books" + "Plato put words..."
        if subject_end_idx > 0 && subject_end_idx <= conj_idx {
            let subject = &words[..subject_end_idx];
            let first_sentence = before_conj.to_vec();
            let mut second_sentence = subject.to_vec();
            second_sentence.extend_from_slice(after_conj);

            println!("DEBUG: Trying subject sharing at conjunction {}", conj_idx);
            println!("  Subject: {:?}", subject);
            println!("  First part: {:?}", first_sentence);
            println!("  Second part: {:?}", second_sentence);

            // Try to match each part with any pattern
            let mut first_match = None;
            let mut second_match = None;
            let mut first_pattern_name = String::new();
            let mut second_pattern_name = String::new();

            for pattern in sorted_patterns.iter() {
                let pattern_tokens = parse_pattern(&pattern.pattern);

                if first_match.is_none() {
                    if let Some(captures) =
                        try_match_pattern(&first_sentence, &pattern_tokens, &app)
                    {
                        first_match = Some((captures, pattern.template.clone()));
                        first_pattern_name = pattern.name.clone();
                        println!("  First part matched pattern: {}", pattern.name);
                    }
                }

                if second_match.is_none() {
                    if let Some(captures) =
                        try_match_pattern(&second_sentence, &pattern_tokens, &app)
                    {
                        second_match = Some((captures, pattern.template.clone()));
                        second_pattern_name = pattern.name.clone();
                        println!("  Second part matched pattern: {}", pattern.name);
                    }
                }

                if first_match.is_some() && second_match.is_some() {
                    break;
                }
            }

            // If both parts matched, generate output
            if let (
                Some((first_captures, first_template)),
                Some((second_captures, second_template)),
            ) = (first_match, second_match)
            {
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

        // Try simple conjunction split (e.g., "dog is black and white")
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

                    if let Some(captures) =
                        try_match_pattern(&first_sentence, &pattern_tokens, &app)
                    {
                        outputs.extend(apply_template(&captures, &pattern.template));
                    }
                    if let Some(captures) =
                        try_match_pattern(&second_sentence, &pattern_tokens, &app)
                    {
                        outputs.extend(apply_template(&captures, &pattern.template));
                    }

                    return outputs.join("\n") + "\n";
                }
            }
        }
    }

    // Single pattern matching fallback
    for pattern in sorted_patterns {
        let pattern_tokens = parse_pattern(&pattern.pattern);
        println!(
            "DEBUG: Fallback - trying pattern '{}' with tokens: {:?}",
            pattern.name, pattern_tokens
        );

        // Try exact match (from beginning)
        if let Some(captures) = try_match_pattern(&words, &pattern_tokens, &app) {
            println!("DEBUG: EXACT MATCH! Captures: {:?}", captures);
            let prolog_outputs = apply_template(&captures, &pattern.template);
            let output = prolog_outputs.join("\n");
            return format!(
                "// FROM: {}\n// PATTERN: {}\n{}\n",
                sentence, pattern.name, output
            );
        }

        // Try substring match (from any position)
        if let Some((captures, start_idx)) =
            try_match_pattern_substring(&words, &pattern_tokens, &app)
        {
            println!(
                "DEBUG: SUBSTRING MATCH at index {}! Captures: {:?}",
                start_idx, captures
            );
            let prolog_outputs = apply_template(&captures, &pattern.template);
            let output = prolog_outputs.join("\n");
            return format!(
                "// FROM: {}\n// PATTERN: {} (substring match at word {})\n{}\n",
                sentence, pattern.name, start_idx, output
            );
        }
    }

    app.logger
        .log(
            LogLevel::Warning,
            "UNPARSED_SENTENCE",
            "No pattern matched for sentence",
            Some(sentence),
        )
        .ok();

    format!(
        "// FROM: {}\n// WARNING: No pattern matched\nprolog_fact('{}')\n",
        sentence,
        sentence.replace("'", "\\'")
    )
}

pub fn parse_input(app: &mut PrologApp, input: &String) -> String {
    let sentences = parse_sentences(input);
    let parsed_sentences: Vec<String> = sentences.iter().map(|s| parse_prolog(app, s)).collect();
    parsed_sentences.join("\n\n")
}
