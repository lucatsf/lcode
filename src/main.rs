use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        // A configuração da janela agora é feita através do campo `viewport`
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(800.0, 600.0)) // Define o tamanho inicial da janela
            .with_title("lcode"), // Define o título da janela
        ..Default::default()
    };
    eframe::run_native(
        "lcode",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello, lcode!");
            ui.label("Bem-vindo ao seu editor de código minimalista e de alta performance.");
        });
    }
}