use crate::app::parser::{self};

const MIDDLE_GAP: f32 = 20.0;
const BOTTOM_GAP: f32 = 35.0;

#[derive(Default)]
pub struct PrologApp {
    input_text: String,
    parsed_output: String,
}



impl eframe::App for PrologApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {            
            let available_height = ui.available_height();
            let panel_width = (ui.available_width() - MIDDLE_GAP) / 2.0;
            
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(panel_width, available_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.heading("Input Text");
                        ui.separator();

                        let text_height = ui.available_height() - BOTTOM_GAP;

                        let response = ui.add_sized(
                            [ui.available_width(), text_height.max(100.0)],
                            egui::TextEdit::multiline(&mut self.input_text)
                                .hint_text("Enter natural language text here...\n\nExample:\nBear is an animal\nCat is a mammal\nMammals are animals")
                        );
                        
                        if response.changed() {
                            self.update_parsed_output();
                        }
                        
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
                        ui.heading("Output Text");
                        ui.separator();

                        let text_height = ui.available_height() - BOTTOM_GAP;

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add_sized(
                                [ui.available_width(), text_height.max(100.0)],
                                egui::TextEdit::multiline(&mut self.parsed_output)
                                    .interactive(false)
                            );
                        });
                        
                        ui.separator();

                        if ui.button("Copy Output Text").clicked() {
                            ui.output_mut(|o| o.copied_text = self.parsed_output.clone());
                        }
                    },
                );
            });
        });
    }
}

impl PrologApp {
    pub fn with_text(text: String) -> Self {
        let mut app = Self {
            input_text: text,
            parsed_output: String::new(),
        };
        app.update_parsed_output();
        app
    }

    fn update_parsed_output(&mut self) {
        if self.input_text.is_empty() {
            self.parsed_output = "// Parsed Prolog code will appear here...".to_string();
        } else {
            let parse_result = parser::parse_input(&self.input_text);
            self.parsed_output = parse_result;
        }
    }
}