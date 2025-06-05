use eframe::egui;
use ropey::Rope;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use memmap2::Mmap;

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

// Constantes para otimização da renderização
const LINE_HEIGHT: f32 = 16.0;
const LINE_NUMBER_GUTTER_WIDTH: f32 = 60.0;
const SIDE_PANEL_WIDTH: f32 = 200.0;

struct MyApp {
    editor_content: Rope,
    file_path: Option<String>,
}

impl Default for MyApp {
    fn default() -> Self {
        let initial_text = "Hello, lcode!\n\nEste é o nosso editor de código minimalista.\n\nTentando carregar um arquivo grande...\n".to_string();
        let mut app = Self {
            editor_content: Rope::from(initial_text),
            file_path: None,
        };

        let test_path = Path::new("test_large_file.txt");
        match load_file_into_rope(test_path) {
            Ok(rope) => {
                app.editor_content = rope;
                app.file_path = Some(test_path.to_string_lossy().into_owned());
                eprintln!("Arquivo '{}' carregado com sucesso!", test_path.display());
            },
            Err(e) => {
                eprintln!("Erro ao carregar o arquivo '{}': {}", test_path.display(), e);
                eprintln!("Usando conteúdo inicial padrão.");
            }
        }
        app
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("file_explorer_panel")
            .min_width(SIDE_PANEL_WIDTH)
            .default_width(SIDE_PANEL_WIDTH)
            .show(ctx, |ui| {
                ui.heading("Explorador de Arquivos");
                ui.separator();
                ui.label("Conteúdo do diretório virá aqui.");
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Editor: {}", self.file_path.as_deref().unwrap_or("[No File Loaded]")));
            ui.separator();

            // Total de linhas no Rope
            let total_lines = self.editor_content.len_lines();

            egui::ScrollArea::vertical().show_rows(ui, LINE_HEIGHT, total_lines, |ui, row_range| {
                ui.horizontal(|ui| {
                    // Gutter para números de linha
                    ui.vertical(|ui| {
                        ui.set_width(LINE_NUMBER_GUTTER_WIDTH);
                        ui.spacing_mut().item_spacing.y = 0.0;

                        // Itera sobre o range de linhas visíveis para exibir os números
                        for i in row_range.start..row_range.end {
                            ui.monospace(format!("{:>4}", i + 1));
                        }
                    });

                    // Painel de texto do editor
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width()); // Garante que o painel ocupe toda a largura restante
                        ui.spacing_mut().item_spacing.y = 0.0;

                        // Itera sobre as linhas do Ropey para exibir o texto
                        // O `lines_at` e `take` já garantem a virtualização.
                        // Usamos `line.trim_end_matches('\n').trim_end_matches('\r')`
                        // para remover as quebras de linha que o ropey inclui.
                        for line_ropey in self.editor_content.lines_at(row_range.start).take(row_range.len()) {
                            let line_str = line_ropey.as_str().unwrap_or("");
                            // Remove o caractere de nova linha para não afetar o layout e garantir que
                            // cada `label` represente puramente o conteúdo da linha.
                            let trimmed_line = line_str.trim_end_matches('\n').trim_end_matches('\r');
                            ui.monospace(trimmed_line);
                        }
                    });
                });
            });
        });
    }
}

// Função para carregar um arquivo no Rope usando mmap (inalterada)
fn load_file_into_rope(path: &Path) -> io::Result<Rope> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_len = metadata.len();

    if file_len < 1024 * 1024 {
        let mut buffer = String::new();
        file.take(file_len).read_to_string(&mut buffer)?;
        Ok(Rope::from(buffer))
    } else {
        #[cfg(target_family = "unix")]
        {
            let mmap = unsafe { Mmap::map(&file)? };
            let content_str = std::str::from_utf8(&mmap)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Arquivo não é UTF-8 válido: {}", e)))?;
            Ok(Rope::from(content_str))
        }
        #[cfg(not(target_family = "unix"))]
        {
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;
            Ok(Rope::from(buffer))
        }
    }
}