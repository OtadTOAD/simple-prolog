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

use super::pattern_matcher::{apply_template, parse_pattern, try_match_pattern};

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
    for pattern in sorted_patterns {
        let pattern_tokens = parse_pattern(&pattern.pattern);

        if try_match_pattern(&words, &pattern_tokens, app) {
            let prolog_output = apply_template(&words, &pattern.template);
            return format!(
                "// FROM: {}\n// PATTERN: {}\n{}\n",
                sentence, pattern.name, prolog_output
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
