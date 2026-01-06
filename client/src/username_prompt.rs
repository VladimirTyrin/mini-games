use eframe::egui;
use std::sync::{Arc, Mutex};

struct UsernamePromptApp {
    username: String,
    result: Arc<Mutex<Option<String>>>,
    focus_requested: bool,
}

impl UsernamePromptApp {
    fn new(result: Arc<Mutex<Option<String>>>) -> Self {
        Self {
            username: String::new(),
            result,
            focus_requested: false,
        }
    }

    fn submit(&mut self, ctx: &egui::Context) {
        let trimmed = self.username.trim().to_string();
        if !trimmed.is_empty() {
            *self.result.lock().unwrap() = Some(trimmed);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for UsernamePromptApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading("Welcome to Mini Games!");
                ui.add_space(20.0);

                ui.label("Please enter your username:");
                ui.add_space(10.0);

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.username)
                        .hint_text("Username")
                        .desired_width(200.0)
                );

                if !self.focus_requested {
                    response.request_focus();
                    self.focus_requested = true;
                }

                let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                ui.add_space(15.0);

                let can_submit = !self.username.trim().is_empty();
                let button_clicked = ui.add_enabled(can_submit, egui::Button::new("Continue (Enter)")).clicked();

                if can_submit && (enter_pressed || button_clicked) {
                    self.submit(ctx);
                }

                if !can_submit {
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("Username cannot be empty").color(egui::Color32::GRAY).small());
                }
            });
        });
    }
}

pub fn prompt_for_username() -> Option<String> {
    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([300.0, 180.0])
            .with_min_inner_size([300.0, 180.0])
            .with_max_inner_size([300.0, 180.0])
            .with_resizable(false)
            .with_title("Enter Username"),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Enter Username",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(UsernamePromptApp::new(result_clone)))
        }),
    );

    Arc::try_unwrap(result)
        .ok()
        .and_then(|mutex| mutex.into_inner().ok())
        .flatten()
}
