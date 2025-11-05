use crate::app::database::WordType;

#[derive(Debug, Clone)]
pub struct TokenHighlight {
    pub word: String,
    pub word_index: usize,
    pub capture_index: usize,
    pub token_type: TokenType,
    //pub is_editable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Noun,
    Verb,
    Adjective,
    Other(WordType),
    Greedy,
}

#[derive(Debug, Clone)]
pub struct SentenceMatch {
    pub words: Vec<String>,
    pub pattern_name: String,
    pub template: String,
    pub highlights: Vec<TokenHighlight>,
    pub generated_output: String,
}

impl SentenceMatch {
    pub fn regenerate_output(&mut self) {
        let mut captures: Vec<String> = vec![String::new(); self.highlights.len()];

        for highlight in &self.highlights {
            if highlight.capture_index > 0 && highlight.capture_index <= captures.len() {
                captures[highlight.capture_index - 1] = highlight.word.clone();
            }
        }

        self.generated_output = apply_template_simple(&captures, &self.template);
    }
}

fn apply_template_simple(captures: &[String], template: &str) -> String {
    let mut result = template.to_string();

    for (i, word) in captures.iter().enumerate() {
        let placeholder = format!("${}", i + 1);
        result = result.replace(&placeholder, word);
    }

    result
}

#[derive(Default)]
pub struct InteractiveParser {
    pub matches: Vec<SentenceMatch>,
    pub dragging_highlight: Option<(usize, usize)>,
    pub temp_selected_word: Option<String>,
    pub selection_start_pos: Option<usize>,
}

impl InteractiveParser {
    pub fn new() -> Self {
        Self {
            matches: Vec::new(),
            dragging_highlight: None,
            temp_selected_word: None,
            selection_start_pos: None,
        }
    }

    pub fn clear(&mut self) {
        self.matches.clear();
        self.dragging_highlight = None;
        self.temp_selected_word = None;
        self.selection_start_pos = None;
    }
}
