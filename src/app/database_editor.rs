use crate::app::database::{Database, PrologPattern, WordEntry, WordType};
use std::sync::{
    Arc, RwLock,
    mpsc::{Receiver, Sender, channel},
};

const DATABASE_JSON_PATH: &str = "prolog_database.json";
const DATABASE_BIN_PATH: &str = "prolog_database.bin";

enum OperationResult {
    SaveComplete(Result<(), String>),
}

pub struct DatabaseEditor {
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

    pattern_page: usize,
    patterns_per_page: usize,
    pattern_search: String,
    cached_pattern_search: String,
    cached_pattern_results: Vec<usize>,

    edit_pattern_index: Option<usize>,
    edit_pattern_name: String,
    edit_pattern_pattern: String,
    edit_pattern_template: String,
    edit_pattern_priority: String,

    operation_sender: Option<Sender<OperationResult>>,
    operation_receiver: Option<Receiver<OperationResult>>,
    is_saving: bool,
    is_adding_word: bool,
    is_adding_pattern: bool,
}

impl Default for WordType {
    fn default() -> Self {
        WordType::Noun
    }
}

impl DatabaseEditor {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            word_search: String::new(),
            word_page: 0,
            words_per_page: 50,
            cached_search: String::from("\x00__UNINITIALIZED__"),
            cached_results: Vec::new(),
            new_word_lemma: String::new(),
            new_word_type: WordType::Noun,
            new_word_forms: String::new(),
            new_pattern_name: String::new(),
            new_pattern_pattern: String::new(),
            new_pattern_template: String::new(),
            new_pattern_priority: String::new(),
            status_message: String::new(),
            pattern_page: 0,
            patterns_per_page: 10,
            pattern_search: String::new(),
            cached_pattern_search: String::from("\x00__UNINITIALIZED__"),
            cached_pattern_results: Vec::new(),
            edit_pattern_index: None,
            edit_pattern_name: String::new(),
            edit_pattern_pattern: String::new(),
            edit_pattern_template: String::new(),
            edit_pattern_priority: String::new(),
            operation_sender: Some(sender),
            operation_receiver: Some(receiver),
            is_saving: false,
            is_adding_word: false,
            is_adding_pattern: false,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, database: &Arc<RwLock<Database>>) {
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

                if let Some(receiver) = &self.operation_receiver {
                    if let Ok(result) = receiver.try_recv() {
                        match result {
                            OperationResult::SaveComplete(Ok(())) => {
                                self.status_message =
                                    "✅ Database saved (JSON + Binary)!".to_string();
                                self.is_saving = false;
                            }
                            OperationResult::SaveComplete(Err(e)) => {
                                self.status_message = format!("❌ Error saving: {}", e);
                                self.is_saving = false;
                            }
                        }
                        ctx.request_repaint();
                    }
                }

                ui.horizontal(|ui| {
                    let save_button =
                        ui.add_enabled(!self.is_saving, egui::Button::new("💾 Save Database"));

                    if self.is_saving {
                        ui.spinner();
                        ui.label("Saving...");
                        ctx.request_repaint();
                    }

                    if save_button.clicked() {
                        self.is_saving = true;
                        self.status_message.clear();

                        let sender = self.operation_sender.clone().unwrap();

                        let db = Arc::clone(database);
                        std::thread::spawn(move || {
                            if let Ok(db_guard) = db.read() {
                                let json_result = db_guard.save(DATABASE_JSON_PATH);
                                let bin_result = db_guard.save(DATABASE_BIN_PATH);

                                let result = match (json_result, bin_result) {
                                    (Ok(_), Ok(_)) => Ok(()),
                                    (Err(e), _) | (_, Err(e)) => Err(e.to_string()),
                                };

                                let _ = sender.send(OperationResult::SaveComplete(result));
                            } else {
                                let _ = sender.send(OperationResult::SaveComplete(Err(
                                    "Failed to lock database".to_string(),
                                )));
                            }
                        });

                        ctx.request_repaint();
                    }
                });
            });
        });
    }

    fn show_word_list(&mut self, ui: &mut egui::Ui, database: &Arc<RwLock<Database>>) {
        if let Ok(read_database) = database.read() {
            ui.horizontal(|ui| {
                ui.label(format!("Total words: {}", read_database.words.len()));
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

            if read_database.words.is_empty() {
                ui.label("No words in database yet.");
                return;
            }

            let search_lower = self.word_search.to_lowercase();

            let filtered_indices: &[usize] = if search_lower != self.cached_search {
                self.cached_search = search_lower.clone();

                if search_lower.is_empty() {
                    self.cached_results = (0..read_database.words.len()).collect();
                } else {
                    self.cached_results = read_database
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
                .id_source("word_list_scroll")
                .max_height(300.0)
                .show(ui, |ui| {
                    use egui::*;

                    for (display_idx, &idx) in page_indices.iter().enumerate() {
                        if let Some(entry) = read_database.words.get(idx) {
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

                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                ui.add_space(16.0);
                                                if ui
                                                    .button(RichText::new("🗑").size(14.0))
                                                    .clicked()
                                                {
                                                    to_remove.push(idx);
                                                }
                                            },
                                        );
                                    });
                                });
                        }
                    }
                });

            if !to_remove.is_empty() {
                if let Ok(mut write_database) = database.write() {
                    for idx in to_remove.iter().rev() {
                        write_database.words.remove(*idx);
                    }
                    write_database.rebuild_index();
                    self.status_message = format!("Removed {} word(s)", to_remove.len());
                    self.cached_search.clear();
                }
            }
        } else {
            ui.label("Error: Could not access database");
        }
    }
    fn show_word_form(&mut self, ui: &mut egui::Ui, database: &Arc<RwLock<Database>>) {
        ui.horizontal(|ui| {
            ui.label("Lemma:");
            ui.add(
                egui::TextEdit::singleline(&mut self.new_word_lemma)
                    .desired_width(ui.available_width()),
            );
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
                    ui.selectable_value(&mut self.new_word_type, WordType::Determiner, "Determiner")
                });
        });

        ui.horizontal(|ui| {
            ui.label("Forms (comma-separated):");
            ui.add(
                egui::TextEdit::singleline(&mut self.new_word_forms)
                    .desired_width(ui.available_width()),
            );
        });

        ui.horizontal(|ui| {
            let add_button = ui.add_enabled(!self.is_adding_word, egui::Button::new("Add Word"));

            if self.is_adding_word {
                ui.spinner();
                ui.label("Adding word...");
            }

            if add_button.clicked() && !self.new_word_lemma.is_empty() {
                self.is_adding_word = true;
                self.status_message.clear();

                // Prepare data for background thread
                let forms: Vec<String> = self
                    .new_word_forms
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                let lemma = self.new_word_lemma.clone();
                let entry = WordEntry {
                    lemma: lemma.clone(),
                    word_type: self.new_word_type.clone(),
                    forms,
                };

                // Add to database immediately (adding words is fast)
                if let Ok(mut write_database) = database.write() {
                    write_database.words.push(entry);
                    write_database.rebuild_index();
                    self.status_message = format!("✅ Added word: {}", lemma);
                    self.cached_search.clear();
                }

                self.new_word_lemma.clear();
                self.new_word_forms.clear();
                self.is_adding_word = false;
            }
        });
    }

    fn show_pattern_list(&mut self, ui: &mut egui::Ui, database: &Arc<RwLock<Database>>) {
        let Ok(read_database) = database.read() else {
            ui.label("Error: Could not access database");
            return;
        };

        // Search bar
        ui.horizontal(|ui| {
            ui.label(format!("Total patterns: {}", read_database.patterns.len()));
            ui.separator();
            ui.label("Search:");
            let search_response = ui.add(
                egui::TextEdit::singleline(&mut self.pattern_search)
                    .hint_text("Search patterns...")
                    .desired_width(ui.available_width() - 20.0),
            );
            if search_response.changed() {
                self.pattern_page = 0;
                self.cached_pattern_search.clear();
            }
        });

        if read_database.patterns.is_empty() {
            ui.label("No patterns in database yet.");
            return;
        }

        let search_lower = self.pattern_search.to_lowercase();

        // Filter patterns
        let filtered_indices: &[usize] = if search_lower != self.cached_pattern_search {
            self.cached_pattern_search = search_lower.clone();

            if search_lower.is_empty() {
                self.cached_pattern_results = (0..read_database.patterns.len()).collect();
            } else {
                self.cached_pattern_results = read_database
                    .patterns
                    .iter()
                    .enumerate()
                    .filter(|(_, pattern)| {
                        pattern.name.to_lowercase().contains(&search_lower)
                            || pattern.pattern.to_lowercase().contains(&search_lower)
                            || pattern.template.to_lowercase().contains(&search_lower)
                    })
                    .map(|(idx, _)| idx)
                    .collect();
            }
            &self.cached_pattern_results
        } else {
            &self.cached_pattern_results
        };

        let total_filtered = filtered_indices.len();
        let total_pages = (total_filtered + self.patterns_per_page - 1) / self.patterns_per_page;

        if self.pattern_page >= total_pages && total_pages > 0 {
            self.pattern_page = total_pages - 1;
        }

        let start = self.pattern_page * self.patterns_per_page;
        let end = (start + self.patterns_per_page).min(total_filtered);
        let page_indices = &filtered_indices[start..end];

        // Pagination controls
        ui.horizontal(|ui| {
            if total_filtered > self.patterns_per_page {
                ui.label(format!("{}-{} of {}", start + 1, end, total_filtered));
            } else {
                ui.label(format!("{} matches", total_filtered));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if total_pages > 1 {
                    if ui.button("⏭").clicked() {
                        self.pattern_page = total_pages - 1;
                    }
                    if ui.button("▶").clicked() && self.pattern_page < total_pages - 1 {
                        self.pattern_page += 1;
                    }
                    ui.label(format!("{}/{}", self.pattern_page + 1, total_pages));
                    if ui.button("◀").clicked() && self.pattern_page > 0 {
                        self.pattern_page -= 1;
                    }
                    if ui.button("⏮").clicked() {
                        self.pattern_page = 0;
                    }
                }
            });
        });

        let mut to_remove = Vec::new();
        let mut to_toggle = Vec::new();
        let mut save_edit: Option<(usize, String, String, String, i32)> = None;
        let mut cancel_edit = false;
        let mut start_edit: Option<(usize, String, String, String, i32)> = None;

        egui::ScrollArea::vertical()
            .id_source("pattern_list_scroll")
            .max_height(300.0)
            .show(ui, |ui| {
                for (display_idx, &idx) in page_indices.iter().enumerate() {
                    if let Some(pattern) = read_database.patterns.get(idx) {
                        let bg_color = if display_idx % 2 == 0 {
                            egui::Color32::from_rgb(34, 34, 34)
                        } else {
                            egui::Color32::from_rgb(40, 40, 40)
                        };

                        let is_editing = self.edit_pattern_index == Some(idx);

                        egui::Frame::none()
                            .fill(bg_color)
                            .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                            .show(ui, |ui| {
                                if is_editing {
                                    // Edit mode
                                    ui.horizontal(|ui| {
                                        ui.label("Name:");
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.edit_pattern_name)
                                                .desired_width(ui.available_width()),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Pattern:");
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.edit_pattern_pattern,
                                            )
                                            .desired_width(ui.available_width()),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Template:");
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.edit_pattern_template,
                                            )
                                            .desired_width(ui.available_width()),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Priority:");
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.edit_pattern_priority,
                                            )
                                            .desired_width(100.0),
                                        );

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("❌ Cancel").clicked() {
                                                    cancel_edit = true;
                                                }

                                                if ui.button("💾 Save").clicked() {
                                                    let priority = self
                                                        .edit_pattern_priority
                                                        .parse()
                                                        .unwrap_or(50);
                                                    save_edit = Some((
                                                        idx,
                                                        self.edit_pattern_name.clone(),
                                                        self.edit_pattern_pattern.clone(),
                                                        self.edit_pattern_template.clone(),
                                                        priority,
                                                    ));
                                                }
                                            },
                                        );
                                    });
                                } else {
                                    // View mode
                                    ui.horizontal(|ui| {
                                        let status = if pattern.enabled { "Y" } else { "N" };
                                        let status_color = if pattern.enabled {
                                            egui::Color32::from_rgb(50, 200, 50)
                                        } else {
                                            egui::Color32::from_rgb(200, 50, 50)
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
                                                .color(egui::Color32::from_rgb(138, 138, 138))
                                                .size(13.0),
                                        );

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.small_button("🗑").clicked() {
                                                    to_remove.push(idx);
                                                }

                                                if ui.small_button("✏").clicked() {
                                                    start_edit = Some((
                                                        idx,
                                                        pattern.name.clone(),
                                                        pattern.pattern.clone(),
                                                        pattern.template.clone(),
                                                        pattern.priority,
                                                    ));
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
                                                .color(egui::Color32::from_rgb(100, 100, 100))
                                                .size(11.0),
                                        );
                                        ui.monospace(
                                            egui::RichText::new(&pattern.pattern)
                                                .color(egui::Color32::from_rgb(150, 150, 150))
                                                .size(11.0),
                                        );
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("  Template:")
                                                .color(egui::Color32::from_rgb(100, 100, 100))
                                                .size(11.0),
                                        );
                                        ui.monospace(
                                            egui::RichText::new(&pattern.template)
                                                .color(egui::Color32::from_rgb(150, 150, 150))
                                                .size(11.0),
                                        );
                                    });
                                }
                            });

                        ui.add_space(2.0);
                    }
                }
            });

        // Drop the read guard before acquiring write guard
        drop(read_database);

        // Handle edit operations
        if cancel_edit {
            self.edit_pattern_index = None;
            self.edit_pattern_name.clear();
            self.edit_pattern_pattern.clear();
            self.edit_pattern_template.clear();
            self.edit_pattern_priority.clear();
        }

        if let Some((idx, name, pattern, template, priority)) = start_edit {
            self.edit_pattern_index = Some(idx);
            self.edit_pattern_name = name;
            self.edit_pattern_pattern = pattern;
            self.edit_pattern_template = template;
            self.edit_pattern_priority = priority.to_string();
        }

        if let Some((idx, name, pattern, template, priority)) = save_edit {
            if let Ok(mut write_database) = database.write() {
                if let Some(p) = write_database.patterns.get_mut(idx) {
                    p.name = name;
                    p.pattern = pattern;
                    p.template = template;
                    p.priority = priority;
                    self.status_message = "✅ Pattern updated".to_string();
                }
            }
            self.edit_pattern_index = None;
            self.edit_pattern_name.clear();
            self.edit_pattern_pattern.clear();
            self.edit_pattern_template.clear();
            self.edit_pattern_priority.clear();
        }

        // Handle toggle and remove operations
        if !to_toggle.is_empty() || !to_remove.is_empty() {
            if let Ok(mut write_database) = database.write() {
                for idx in to_toggle {
                    if let Some(pattern) = write_database.patterns.get_mut(idx) {
                        pattern.enabled = !pattern.enabled;
                    }
                }

                for idx in to_remove.iter().rev() {
                    write_database.patterns.remove(*idx);
                    self.status_message = "Removed pattern".to_string();
                }
                self.cached_pattern_search.clear();
            }
        }
    }

    fn show_pattern_form(&mut self, ui: &mut egui::Ui, database: &Arc<RwLock<Database>>) {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(
                egui::TextEdit::singleline(&mut self.new_pattern_name)
                    .desired_width(ui.available_width()),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Regex Pattern:");
            ui.add(
                egui::TextEdit::singleline(&mut self.new_pattern_pattern)
                    .desired_width(ui.available_width()),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Template:");
            ui.add(
                egui::TextEdit::singleline(&mut self.new_pattern_template)
                    .desired_width(ui.available_width()),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Priority:");
            ui.add(
                egui::TextEdit::singleline(&mut self.new_pattern_priority)
                    .desired_width(ui.available_width()),
            );
        });

        ui.label(
            egui::RichText::new("Tip: Use $1, $2, etc. in template for capture groups")
                .italics()
                .color(egui::Color32::from_rgb(100, 100, 100))
                .size(11.0),
        );

        ui.horizontal(|ui| {
            let add_button =
                ui.add_enabled(!self.is_adding_pattern, egui::Button::new("Add Pattern"));

            if self.is_adding_pattern {
                ui.spinner();
                ui.label("Adding pattern...");
            }

            if add_button.clicked()
                && !self.new_pattern_name.is_empty()
                && !self.new_pattern_pattern.is_empty()
            {
                self.is_adding_pattern = true;
                self.status_message.clear();

                // Add pattern immediately (adding patterns is fast)
                let priority: i32 = self.new_pattern_priority.parse().unwrap_or(50);

                let pattern = PrologPattern {
                    name: self.new_pattern_name.clone(),
                    pattern: self.new_pattern_pattern.clone(),
                    template: self.new_pattern_template.clone(),
                    priority,
                    enabled: true,
                };

                if let Ok(mut write_database) = database.write() {
                    write_database.patterns.push(pattern);
                    self.status_message = format!("✅ Added pattern: {}", self.new_pattern_name);
                }

                self.new_pattern_name.clear();
                self.new_pattern_pattern.clear();
                self.new_pattern_template.clear();
                self.new_pattern_priority.clear();
                self.is_adding_pattern = false;
            }
        });
    }
}
