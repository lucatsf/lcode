// src/main.rs

use lcode::MyApp; // Importa a struct MyApp de nossa biblioteca

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(1200.0, 800.0))
            .with_title("lcode"),
        ..Default::default()
    };
    eframe::run_native(
        "lcode",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}