use eframe::egui;
use ropey::Rope; // Importa a crate Ropey

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(800.0, 600.0))
            .with_title("lcode"),
        ..Default::default()
    };
    eframe::run_native(
        "lcode",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    editor_content: Rope, // O buffer de texto do editor, usando Ropey
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            // Por enquanto, inicializamos com um texto simples.
            // No futuro, isso será carregado de um arquivo.
            editor_content: Rope::from("Hello, lcode!\n\nEste é o nosso editor de código minimalista.\nComeçando a integrar o Ropey para gerenciar o texto."),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Editor Content:");
            // Exibe o conteúdo do Rope.
            // Isso é uma exibição bem básica e será otimizada na Fase 2.
            for line in self.editor_content.lines() {
                ui.label(line.as_str().unwrap_or("")); // Converte cada linha para &str para exibição
            }
        });
    }
}