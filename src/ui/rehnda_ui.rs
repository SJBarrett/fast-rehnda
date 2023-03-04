pub struct RehndaUi {
    name: String,
    age: u32,
}

impl Default for RehndaUi {
    fn default() -> Self {
        RehndaUi {
            name: "".to_string(),
            age: 1,
        }
    }
}

impl RehndaUi {
    pub fn ui(&mut self, egui_ctx: &egui::Context)  {
        egui::Window::new("My window").show(egui_ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name);
            });
            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            if ui.button("Click each year").clicked() {
                self.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", self.name, self.age));
        });
    }
}