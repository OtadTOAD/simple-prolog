use std::{path::Path, sync::{Arc, RwLock}};

use crate::app::{database::Database, database_editor::DatabaseEditor, parser, interactive_parser::InteractiveParser, query_engine::QueryEngine};

const DATABASE_PATH: &str = "prolog_database.bin";
const BOTTOM_GAP: f32 = 35.0;

#[derive(PartialEq)]
enum AppTab {
    Parser,
    DatabaseEditor,
}

pub struct PrologApp {
    input_text: String,
    parsed_output: String,
    query_text: String,
    query_results: String,

    pub database: Arc<RwLock<Database>>,
    pub interactive_parser: InteractiveParser,
    pub query_engine: QueryEngine,
    
    current_tab: AppTab,
    database_editor: DatabaseEditor,
}

impl Default for PrologApp {
    fn default() -> Self {
        let database = Database::new(Path::new(DATABASE_PATH)).unwrap();
        let mut query_engine = QueryEngine::new();
        
        // Try to load query config file
        if let Err(e) = query_engine.load_config_file("query_config.txt") {
            eprintln!("Note: Could not load query_config.txt: {}", e);
            eprintln!("You can create this file to define custom rules and patterns.");
        }
        
        Self {
            input_text: String::new(),
            parsed_output: "// Parsed Prolog code will appear here...".to_string(),
            query_text: String::new(),
            query_results: "// Query results will appear here...".to_string(),
            database: Arc::new(RwLock::new(database)),
            current_tab: AppTab::Parser,
            database_editor: DatabaseEditor::new(),
            interactive_parser: InteractiveParser::new(),
            query_engine,
        }
    }
}

impl eframe::App for PrologApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, AppTab::Parser, "📝 Parser");
                ui.selectable_value(&mut self.current_tab, AppTab::DatabaseEditor, "🗄 Database Editor");
            });
        });
        
        match self.current_tab {
            AppTab::Parser => self.show_parser_tab(ctx),
            AppTab::DatabaseEditor => self.database_editor.show(ctx, &self.database.clone()),
        }
    }
}

impl PrologApp {
    pub fn with_text(text: String) -> Self {
        let database = Database::new(Path::new(DATABASE_PATH)).unwrap();
        let mut query_engine = QueryEngine::new();
        
        // Try to load query config file
        if let Err(e) = query_engine.load_config_file("query_config.txt") {
            eprintln!("Note: Could not load query_config.txt: {}", e);
        }

        let mut app = Self {
            parsed_output: String::new(),
            input_text: text,
            query_text: String::new(),
            query_results: "// Query results will appear here...".to_string(),
            database: Arc::new(RwLock::new(database)),
            current_tab: AppTab::Parser,
            database_editor: DatabaseEditor::new(),
            interactive_parser: InteractiveParser::new(),
            query_engine,
        };
        app.update_parsed_output();
        app
    }
    
    fn show_parser_tab(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {            
            let available_height = ui.available_height();
            let available_width = ui.available_width();
            let separator_width = ui.spacing().item_spacing.x;
            let total_separator_width = separator_width * 2.0; 
            let usable_width = available_width - total_separator_width - 20.0; 
            let panel_width = usable_width / 3.0 - 3.0;
            
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(panel_width, available_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.heading("Input Text");
                        ui.separator();

                        let text_height = ui.available_height() - BOTTOM_GAP;

                        egui::ScrollArea::vertical()
                            .id_source("input_text_scroll")
                            .max_height(text_height.max(100.0))
                            .show(ui, |ui| {
                                let is_dragging = self.interactive_parser.dragging_highlight.is_some();
                                
                                if is_dragging {
                                    ui.label(egui::RichText::new("Click on words below to select from input text. Hold Shift to select multiple words.")
                                        .italics()
                                        .color(egui::Color32::from_rgb(200, 200, 100)));
                                    ui.add_space(5.0);
                                    
                                    let is_shift_held = ui.input(|i| i.modifiers.shift);
                                    
                                    for line in self.input_text.lines() {
                                        ui.horizontal_wrapped(|ui| {
                                            for word in line.split_whitespace() {
                                                let clean_word = word.trim_end_matches('.');
                                                
                                                let is_selected = self.interactive_parser.temp_selected_word.as_ref()
                                                    .map(|s| s.contains(clean_word))
                                                    .unwrap_or(false);
                                                
                                                let button_color = if is_selected {
                                                    egui::Color32::from_rgb(0, 80, 0)
                                                } else {
                                                    egui::Color32::from_rgb(30, 30, 30)
                                                };
                                                
                                                let button = egui::Button::new(clean_word)
                                                    .fill(button_color);
                                                
                                                let response = ui.add(button);
                                                
                                                if response.hovered() {
                                                    ui.painter().rect_stroke(
                                                        response.rect,
                                                        3.0,
                                                        egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 200, 0)),
                                                    );
                                                }
                                                
                                                if response.clicked() {
                                                    if is_shift_held {
                                                        if let Some(ref mut existing) = self.interactive_parser.temp_selected_word {
                                                            existing.push('_');
                                                            existing.push_str(&clean_word.to_lowercase());
                                                        } else {
                                                            self.interactive_parser.temp_selected_word = Some(clean_word.to_lowercase());
                                                        }
                                                    } else {
                                                        self.interactive_parser.temp_selected_word = Some(clean_word.to_lowercase());
                                                    }
                                                }
                                            }
                                        });
                                    }
                                    
                                    let show_selection_ui = self.interactive_parser.temp_selected_word.is_some();
                                    if show_selection_ui {
                                        ui.add_space(10.0);
                                        let selected_text = self.interactive_parser.temp_selected_word.clone().unwrap_or_default();
                                        
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Selected:")
                                                .strong()
                                                .color(egui::Color32::from_rgb(100, 200, 100)));
                                            ui.label(egui::RichText::new(&selected_text)
                                                .strong()
                                                .color(egui::Color32::from_rgb(200, 200, 200)));
                                            
                                            if ui.button("Clear").clicked() {
                                                self.interactive_parser.temp_selected_word = None;
                                            }
                                            
                                            if ui.button("Apply Selection").clicked() {
                                                if let Some((match_idx, word_idx)) = self.interactive_parser.dragging_highlight {
                                                    if let Some(sentence_match) = self.interactive_parser.matches.get_mut(match_idx) {
                                                        if let Some(word) = self.interactive_parser.temp_selected_word.take() {
                                                            if let Some(highlight) = sentence_match.highlights.iter_mut()
                                                                .find(|h| h.word_index == word_idx) {
                                                                highlight.word = word;
                                                                sentence_match.regenerate_output();
                                                            }
                                                        }
                                                    }
                                                    self.interactive_parser.dragging_highlight = None;
                                                }
                                            }
                                        });
                                    }
                                } else {
                                    let response = ui.add_sized(
                                        [ui.available_width(), text_height.max(100.0)],
                                        egui::TextEdit::multiline(&mut self.input_text)
                                            .hint_text("Enter natural language text here...\n\nExample:\nBear is an animal\nCat is a mammal\nMammals are animals")
                                    );
                                    
                                    if response.changed() {
                                        self.update_parsed_output();
                                    }
                                }
                            });
                        
                        ui.separator();
                        
                        if ui.button("Clear Input Text").clicked() {
                            self.input_text.clear();
                            self.parsed_output.clear();
                        }
                    },
                );
                
                ui.separator();
                
                ui.allocate_ui_with_layout(
                    egui::vec2(panel_width, available_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.heading("Parsing");
                        ui.separator();

                        let text_height = ui.available_height() - BOTTOM_GAP;

                        egui::ScrollArea::vertical()
                            .id_source("interactive_scroll")
                            .max_height(text_height.max(100.0))
                            .show(ui, |ui| {
                                self.show_interactive_matches(ui);
                            });
                        
                        ui.separator();

                        if ui.button("Copy Output Text").clicked() {
                            ui.output_mut(|o| o.copied_text = self.parsed_output.clone());
                        }
                    },
                );
                
                ui.separator();
                
                ui.allocate_ui_with_layout(
                    egui::vec2(panel_width, available_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.heading("Query Executor");
                        ui.separator();
                        
                        ui.label(egui::RichText::new("Enter Prolog query:")
                            .color(egui::Color32::from_rgb(150, 150, 150)));
                        
                        let query_input_height = 60.0;
                        let response = ui.add_sized(
                            [ui.available_width(), query_input_height],
                            egui::TextEdit::multiline(&mut self.query_text)
                                .hint_text("Examples:\nanimal(X).\nis_a(cat, X).\nhas_property(X, Y).")
                        );
                        
                        if response.changed() {
                            self.execute_query();
                        }
                        
                        ui.add_space(5.0);
                        
                        if ui.button("Clear Query").clicked() {
                            self.query_text.clear();
                            self.query_results = "// Query results will appear here...".to_string();
                        }
                        
                        ui.add_space(10.0);
                        ui.separator();
                        
                        ui.label(egui::RichText::new("Results:")
                            .strong()
                            .color(egui::Color32::from_rgb(150, 200, 150)));
                        
                        let results_height = ui.available_height() - BOTTOM_GAP;
                        
                        egui::ScrollArea::vertical()
                            .id_source("query_results_scroll")
                            .max_height(results_height.max(100.0))
                            .show(ui, |ui| {
                                ui.add_sized(
                                    [ui.available_width(), results_height.max(100.0)],
                                    egui::TextEdit::multiline(&mut self.query_results)
                                        .code_editor()
                                );
                            });
                    },
                );
            });
        });
    }
    
    fn show_interactive_matches(&mut self, ui: &mut egui::Ui) {
        if self.interactive_parser.matches.is_empty() {
            ui.label("// Parsed Prolog code will appear here...");
            ui.label("// Highlighted words show captured values");
            ui.label("// Drag highlights to reassign references");
            return;
        }
        
        for (match_idx, sentence_match) in self.interactive_parser.matches.iter().enumerate() {
            ui.push_id(match_idx, |ui| {
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width() - 24.0);
                    
                    ui.label(egui::RichText::new(&sentence_match.pattern_name)
                        .strong()
                        .color(egui::Color32::from_rgb(100, 150, 200)));
                    
                    ui.add_space(5.0);
                    
                    ui.horizontal_wrapped(|ui| {
                        let mut skip_until_idx = 0; 
                        
                        for (word_idx, word) in sentence_match.words.iter().enumerate() {
                            if word_idx < skip_until_idx {
                                continue;
                            }
                            
                            if let Some(highlight) = sentence_match.highlights.iter()
                                .find(|h| h.word_index == word_idx) {
                                
                                let is_selected = self.interactive_parser.dragging_highlight
                                    .map(|(m, w)| m == match_idx && w == word_idx)
                                    .unwrap_or(false);
                                
                                let mut color = match highlight.token_type {
                                    crate::app::interactive_parser::TokenType::Noun => egui::Color32::from_rgb(100, 200, 100),
                                    crate::app::interactive_parser::TokenType::Verb => egui::Color32::from_rgb(200, 100, 100),
                                    crate::app::interactive_parser::TokenType::Adjective => egui::Color32::from_rgb(200, 200, 100),
                                    crate::app::interactive_parser::TokenType::Greedy => egui::Color32::from_rgb(150, 100, 200),
                                    _ => egui::Color32::from_rgb(150, 150, 150),
                                };
                                
                                if is_selected {
                                    color = egui::Color32::from_rgb(
                                        color.r().saturating_add(50),
                                        color.g().saturating_add(50),
                                        color.b().saturating_add(50),
                                    );
                                }
                                
                                let display_text = if highlight.token_type == crate::app::interactive_parser::TokenType::Greedy {
                                    let word_count = highlight.word.split('_').count();
                                    skip_until_idx = word_idx + word_count;
                                    &highlight.word 
                                } else {
                                    word 
                                };
                                
                                let label_text = format!("{}(${})", display_text, highlight.capture_index);
                                
                                let button = egui::Button::new(
                                    egui::RichText::new(&label_text)
                                        .color(color)
                                        .strong()
                                )
                                .fill(egui::Color32::from_rgb(40, 40, 40))
                                .stroke(egui::Stroke::NONE);
                                
                                let response = ui.add(button);
                                
                                if is_selected {
                                    ui.painter().rect_stroke(
                                        response.rect,
                                        3.0,
                                        egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 200, 0)),
                                    );
                                }
                                
                                if response.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                
                                if response.clicked() {
                                    if is_selected {
                                        self.interactive_parser.dragging_highlight = None;
                                        self.interactive_parser.temp_selected_word = None;
                                    } else {
                                        self.interactive_parser.dragging_highlight = Some((match_idx, word_idx));
                                        self.interactive_parser.temp_selected_word = None;
                                    }
                                }
                            } else {
                                ui.label(word);
                            }
                        }
                    });
                    
                    ui.add_space(5.0);
                    
                    ui.label(egui::RichText::new("Output:")
                        .italics()
                        .color(egui::Color32::from_rgb(150, 150, 150)));
                    ui.monospace(&sentence_match.generated_output);
                });
            });
            
            ui.add_space(10.0);
        }
    }
    
    fn update_parsed_output(&mut self) {
        if self.input_text.is_empty() {
            self.parsed_output = "// Parsed Prolog code will appear here...".to_string();
            self.interactive_parser.clear();
            
            let mut new_engine = QueryEngine::new();
            if let Err(e) = new_engine.load_config_file("query_config.txt") {
                eprintln!("Note: Could not load query_config.txt: {}", e);
            }
            self.query_engine = new_engine;
        } else {
            let input = self.input_text.clone();
            let parse_result = parser::parse_input(self, &input);
            self.parsed_output = parse_result;
            
            self.query_engine.load_facts_from_output(&self.parsed_output);
        }
    }
    
    fn execute_query(&mut self) {
        if self.query_text.trim().is_empty() {
            self.query_results = "// Query results will appear here...".to_string();
            return;
        }
        
        let mut query_engine = QueryEngine::new();
        
        let has_fact_lines = self
            .parsed_output
            .lines()
            .any(|l| {
                let t = l.trim();
                !t.is_empty() && !t.starts_with("//")
            });

        if has_fact_lines {
            query_engine.load_facts_from_output(&self.parsed_output);
        }
        
        let mut results = Vec::new();
        let mut errors = Vec::new();
        
        for line in self.query_text.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with("//") || line.starts_with("#") {
                continue;
            }
            
            if line.contains(":-") {
                match query_engine.add_rule(line) {
                    Ok(_) => {
                        results.push(format!("// Rule added: {}", line));
                    }
                    Err(e) => {
                        errors.push(format!("// Error adding rule: {}", e));
                    }
                }
            } else if line.contains("-->") {
                match query_engine.add_pattern(line) {
                    Ok(_) => {
                        results.push(format!("// Pattern added: {}", line));
                    }
                    Err(e) => {
                        errors.push(format!("// Error adding pattern: {}", e));
                    }
                }
            } else if line.ends_with('.') && !line.contains('?') {
                if let Some(fact) = query_engine.parse_fact_public(line) {
                    query_engine.add_fact(fact);
                    results.push(format!("// Fact added: {}", line));
                } else {
                    errors.push(format!("// Error parsing fact: {}", line));
                }
            } else {
                match query_engine.query(line) {
                    Ok(query_results) => {
                        if query_results.is_empty() {
                            results.push(format!("// Query: {}", line));
                            results.push("// No results found.".to_string());
                        } else {
                            results.push(format!("// Query: {}", line));
                            results.extend(query_results);
                        }
                    }
                    Err(err) => {
                        errors.push(format!("// Error in query '{}': {}", line, err));
                    }
                }
            }
        }
        
        let mut output = Vec::new();
        let has_errors = !errors.is_empty();
        if has_errors {
            output.extend(errors);
            output.push("".to_string());
        }
        if !results.is_empty() {
            output.extend(results);
        } else if !has_errors {
            output.push("// No queries or statements found.".to_string());
        }
        
        self.query_results = output.join("\n");
    }
}
