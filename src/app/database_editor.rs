use crate::app::database::{Database, PrologPattern, WordEntry, WordType};

const DATABASE_JSON_PATH: &str = "prolog_database.json";
const DATABASE_BIN_PATH: &str = "prolog_database.bin";

#[derive(Default)]
pub struct DatabaseEditor {
    new_word: String,
    new_word_lemma: String,
    new_word_type: WordType,
    new_word_forms: String,

    new_pattern_name: String,
    new_pattern_pattern: String,
    new_pattern_template: String,
    new_pattern_priority: String,

    status_message: String,

    word_search: String,
    word_page: usize,
    words_per_page: usize,

    cached_search: String,
    cached_results: Vec<usize>,
}

impl Default for WordType {
    fn default() -> Self {
        WordType::Noun
    }
}

impl DatabaseEditor {
    pub fn new() -> Self {
        Self {
            word_search: String::new(),
            word_page: 0,
            words_per_page: 50,
            cached_search: String::new(),
            cached_results: Vec::new(),
            ..Default::default()
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, database: &mut Database) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Database Editor");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.group(|ui| {
                    ui.heading("Words");
                    ui.separator();

                    self.show_word_list(ui, database);

                    ui.add_space(10.0);
                    ui.separator();
                    ui.label("Add New Word:");

                    self.show_word_form(ui, database);
                });

                ui.add_space(20.0);

                ui.group(|ui| {
                    ui.heading("Sentence Patterns");
                    ui.separator();

                    self.show_pattern_list(ui, database);

                    ui.add_space(10.0);
                    ui.separator();
                    ui.label("Add New Pattern:");

                    self.show_pattern_form(ui, database);
                });

                ui.add_space(20.0);

                if !self.status_message.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(0, 180, 0), &self.status_message);
                }

                ui.separator();
                if ui.button("💾 Save Database").clicked() {
                    let json_result = database.save(DATABASE_JSON_PATH);
                    let bin_result = database.save(DATABASE_BIN_PATH);

                    match (json_result, bin_result) {
                        (Ok(_), Ok(_)) => {
                            self.status_message = "Database saved (JSON + Binary)!".to_string();
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            self.status_message = format!("Error saving: {}", e);
                        }
                    }
                }
            });
        });
    }

    fn show_word_list(&mut self, ui: &mut egui::Ui, database: &mut Database) {
        ui.horizontal(|ui| {
            ui.label(format!("Total words: {}", database.words.len()));
            ui.separator();
            ui.label("Search:");
            let search_response = ui.add(
                egui::TextEdit::singleline(&mut self.word_search)
                    .hint_text("Type to filter words...")
                    .desired_width(ui.available_width() - 20.0),
            );
            if search_response.changed() {
                self.word_page = 0;
                self.cached_search.clear();
            }
        });

        if database.words.is_empty() {
            ui.label("No words in database yet.");
            return;
        }

        let search_lower = self.word_search.to_lowercase();

        let filtered_indices: &[usize] = if search_lower != self.cached_search {
            self.cached_search = search_lower.clone();

            if search_lower.is_empty() {
                self.cached_results = (0..database.words.len()).collect();
            } else {
                self.cached_results = database
                    .words
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| {
                        entry.lemma.to_lowercase().contains(&search_lower)
                            || entry
                                .forms
                                .iter()
                                .any(|f| f.to_lowercase().contains(&search_lower))
                    })
                    .map(|(idx, _)| idx)
                    .collect();
            }
            &self.cached_results
        } else {
            &self.cached_results
        };

        let total_filtered = filtered_indices.len();
        let total_pages = (total_filtered + self.words_per_page - 1) / self.words_per_page;

        if self.word_page >= total_pages && total_pages > 0 {
            self.word_page = total_pages - 1;
        }

        let start = self.word_page * self.words_per_page;
        let end = (start + self.words_per_page).min(total_filtered);
        let page_indices = &filtered_indices[start..end];

        ui.horizontal(|ui| {
            if total_filtered > self.words_per_page {
                ui.label(format!("{}-{} of {}", start + 1, end, total_filtered));
            } else {
                ui.label(format!("{} matches", total_filtered));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if total_pages > 1 {
                    if ui.button("⏭").clicked() {
                        self.word_page = total_pages - 1;
                    }
                    if ui.button("▶").clicked() && self.word_page < total_pages - 1 {
                        self.word_page += 1;
                    }
                    ui.label(format!("{}/{}", self.word_page + 1, total_pages));
                    if ui.button("◀").clicked() && self.word_page > 0 {
                        self.word_page -= 1;
                    }
                    if ui.button("⏮").clicked() {
                        self.word_page = 0;
                    }
                }
            });
        });

        let mut to_remove = Vec::new();

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                use egui::*;

                for (display_idx, &idx) in page_indices.iter().enumerate() {
                    if let Some(entry) = database.words.get(idx) {
                        let row_color = if display_idx % 2 == 0 {
                            Color32::from_rgb(34, 34, 34)
                        } else {
                            Color32::from_rgb(40, 40, 40)
                        };

                        Frame::none()
                            .fill(row_color)
                            .inner_margin(Margin::symmetric(8.0, 6.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&entry.lemma)
                                            .strong()
                                            .color(Color32::from_rgb(138, 138, 138))
                                            .size(14.0),
                                    );

                                    ui.label(
                                        RichText::new(format!("=> {}", entry.word_type))
                                            .color(Color32::from_rgb(100, 100, 100))
                                            .size(12.0),
                                    );

                                    if !entry.forms.is_empty() {
                                        let forms_text = if entry.forms.len() <= 10 {
                                            format!("({})", entry.forms.join(", "))
                                        } else {
                                            format!("({} forms)", entry.forms.len())
                                        };

                                        ui.label(
                                            RichText::new(forms_text)
                                                .italics()
                                                .color(Color32::from_rgb(120, 120, 120))
                                                .size(12.0),
                                        );
                                    }

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.add_space(16.0);
                                        if ui.button(RichText::new("🗑").size(14.0)).clicked() {
                                            to_remove.push(idx);
                                        }
                                    });
                                });
                            });
                    }
                }
            });

        if !to_remove.is_empty() {
            for idx in to_remove.iter().rev() {
                database.words.remove(*idx);
            }
            database.rebuild_index();
            self.status_message = format!("Removed {} word(s)", to_remove.len());
            self.cached_search.clear();
        }
    }

    fn show_word_form(&mut self, ui: &mut egui::Ui, database: &mut Database) {
        ui.horizontal(|ui| {
            ui.label("Word:");
            ui.text_edit_singleline(&mut self.new_word);
        });

        ui.horizontal(|ui| {
            ui.label("Lemma:");
            ui.text_edit_singleline(&mut self.new_word_lemma);
        });

        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_label("")
                .selected_text(format!("{}", self.new_word_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.new_word_type, WordType::Noun, "Noun");
                    ui.selectable_value(&mut self.new_word_type, WordType::Verb, "Verb");
                    ui.selectable_value(&mut self.new_word_type, WordType::Adjective, "Adjective");
                    ui.selectable_value(&mut self.new_word_type, WordType::Adverb, "Adverb");
                    ui.selectable_value(&mut self.new_word_type, WordType::Pronoun, "Pronoun");
                    ui.selectable_value(
                        &mut self.new_word_type,
                        WordType::Preposition,
                        "Preposition",
                    );
                    ui.selectable_value(
                        &mut self.new_word_type,
                        WordType::Conjunction,
                        "Conjunction",
                    );
                    ui.selectable_value(
                        &mut self.new_word_type,
                        WordType::Interjection,
                        "Interjection",
                    );
                });
        });

        ui.horizontal(|ui| {
            ui.label("Forms (comma-separated):");
            ui.text_edit_singleline(&mut self.new_word_forms);
        });

        if ui.button("Add Word").clicked() {
            if !self.new_word.is_empty() {
                let forms: Vec<String> = self
                    .new_word_forms
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                let lemma = if self.new_word_lemma.is_empty() {
                    self.new_word.clone()
                } else {
                    self.new_word_lemma.clone()
                };

                let entry = WordEntry {
                    lemma,
                    word_type: self.new_word_type.clone(),
                    forms,
                };

                database.words.push(entry);
                database.rebuild_index();
                self.status_message = format!("Added word: {}", self.new_word);
                self.cached_search.clear();

                self.new_word.clear();
                self.new_word_lemma.clear();
                self.new_word_forms.clear();
            }
        }
    }

    fn show_pattern_list(&mut self, ui: &mut egui::Ui, database: &mut Database) {
        ui.label(format!("Total patterns: {}", database.patterns.len()));

        if database.patterns.is_empty() {
            ui.label("No patterns in database yet.");
            return;
        }

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                let mut to_remove = Vec::new();
                let mut to_toggle = Vec::new();

                for (idx, pattern) in database.patterns.iter().enumerate() {
                    let bg_color = if idx % 2 == 0 {
                        egui::Color32::from_rgb(250, 250, 250)
                    } else {
                        egui::Color32::from_rgb(240, 245, 250)
                    };

                    egui::Frame::none()
                        .fill(bg_color)
                        .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let status = if pattern.enabled { "✓" } else { "✗" };
                                let status_color = if pattern.enabled {
                                    egui::Color32::from_rgb(0, 150, 0)
                                } else {
                                    egui::Color32::from_rgb(150, 0, 0)
                                };

                                ui.label(
                                    egui::RichText::new(status)
                                        .color(status_color)
                                        .strong()
                                        .size(14.0),
                                );

                                ui.label(
                                    egui::RichText::new(format!("[{}]", pattern.priority))
                                        .color(egui::Color32::from_rgb(100, 100, 100))
                                        .size(12.0),
                                );

                                ui.label(
                                    egui::RichText::new(&pattern.name)
                                        .strong()
                                        .color(egui::Color32::from_rgb(20, 20, 80))
                                        .size(13.0),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("🗑").clicked() {
                                            to_remove.push(idx);
                                        }

                                        if ui
                                            .small_button(if pattern.enabled {
                                                "Disable"
                                            } else {
                                                "Enable"
                                            })
                                            .clicked()
                                        {
                                            to_toggle.push(idx);
                                        }
                                    },
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("  Pattern:")
                                        .color(egui::Color32::from_rgb(80, 80, 80))
                                        .size(11.0),
                                );
                                ui.monospace(
                                    egui::RichText::new(&pattern.pattern)
                                        .color(egui::Color32::from_rgb(40, 40, 40))
                                        .size(11.0),
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("  Template:")
                                        .color(egui::Color32::from_rgb(80, 80, 80))
                                        .size(11.0),
                                );
                                ui.monospace(
                                    egui::RichText::new(&pattern.template)
                                        .color(egui::Color32::from_rgb(40, 40, 40))
                                        .size(11.0),
                                );
                            });
                        });

                    ui.add_space(2.0);
                }

                for idx in to_toggle {
                    database.patterns[idx].enabled = !database.patterns[idx].enabled;
                }

                for idx in to_remove.iter().rev() {
                    database.patterns.remove(*idx);
                    self.status_message = "Removed pattern".to_string();
                }
            });
    }

    fn show_pattern_form(&mut self, ui: &mut egui::Ui, database: &mut Database) {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.new_pattern_name);
        });

        ui.horizontal(|ui| {
            ui.label("Regex Pattern:");
            ui.text_edit_singleline(&mut self.new_pattern_pattern);
        });

        ui.horizontal(|ui| {
            ui.label("Template:");
            ui.text_edit_singleline(&mut self.new_pattern_template);
        });

        ui.horizontal(|ui| {
            ui.label("Priority:");
            ui.text_edit_singleline(&mut self.new_pattern_priority);
        });

        ui.label(
            egui::RichText::new("Tip: Use $1, $2, etc. in template for capture groups")
                .italics()
                .color(egui::Color32::from_rgb(100, 100, 100))
                .size(11.0),
        );

        if ui.button("Add Pattern").clicked() {
            if !self.new_pattern_name.is_empty() && !self.new_pattern_pattern.is_empty() {
                let priority: i32 = self.new_pattern_priority.parse().unwrap_or(50);

                let pattern = PrologPattern {
                    name: self.new_pattern_name.clone(),
                    pattern: self.new_pattern_pattern.clone(),
                    template: self.new_pattern_template.clone(),
                    priority,
                    enabled: true,
                };

                database.patterns.push(pattern);
                self.status_message = format!("Added pattern: {}", self.new_pattern_name);

                self.new_pattern_name.clear();
                self.new_pattern_pattern.clear();
                self.new_pattern_template.clear();
                self.new_pattern_priority.clear();
            }
        }
    }
}
