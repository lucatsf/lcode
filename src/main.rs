use eframe::egui;
use ropey::Rope;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use memmap2::Mmap;
use rfd::AsyncFileDialog;
use walkdir::WalkDir;
use pollster;

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

const LINE_HEIGHT: f32 = 16.0;
const LINE_NUMBER_GUTTER_WIDTH: f32 = 60.0;
const SIDE_PANEL_WIDTH: f32 = 200.0;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum FileSystemItem {
    File(PathBuf),
    Directory(PathBuf),
}

struct MyApp {
    editor_content: Rope,
    file_path: Option<String>,
    current_dir: Option<PathBuf>,
    expanded_dirs: HashMap<PathBuf, bool>,
    picked_folder_tx: std::sync::mpsc::Sender<PathBuf>,
    picked_folder_rx: std::sync::mpsc::Receiver<PathBuf>,
}

impl Default for MyApp {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let initial_text = "Hello, lcode!\n\nEste é o nosso editor de código minimalista.\n\nClique em 'Abrir Diretório' para começar.\n".to_string();
        let mut app = Self {
            editor_content: Rope::from(initial_text),
            file_path: None,
            current_dir: None,
            expanded_dirs: HashMap::new(),
            picked_folder_tx: tx,
            picked_folder_rx: rx,
        };

        let test_path = Path::new("test_large_file.txt");
        if test_path.exists() {
            match load_file_into_rope(test_path) {
                Ok(rope) => {
                    app.editor_content = rope;
                    app.file_path = Some(test_path.to_string_lossy().into_owned());
                    eprintln!("Arquivo '{}' carregado com sucesso ao iniciar!", test_path.display());
                },
                Err(e) => {
                    eprintln!("Erro ao carregar o arquivo '{}' ao iniciar: {}", test_path.display(), e);
                }
            }
        } else {
            eprintln!("Arquivo de teste '{}' não encontrado. Usando conteúdo inicial padrão.", test_path.display());
        }
        app
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(path) = self.picked_folder_rx.try_recv() {
            self.current_dir = Some(path);
            self.expanded_dirs.clear();
            eprintln!("Diretório selecionado: {:?}", self.current_dir);
        }

        egui::SidePanel::left("file_explorer_panel")
            .min_width(SIDE_PANEL_WIDTH)
            .default_width(SIDE_PANEL_WIDTH)
            .show(ctx, |ui| {
                ui.heading("Explorador de Arquivos");
                ui.separator();

                if ui.button("Abrir Diretório...").clicked() {
                    let tx = self.picked_folder_tx.clone();
                    std::thread::spawn(move || {
                        let path_handle = pollster::block_on(AsyncFileDialog::new().pick_folder());
                        if let Some(path) = path_handle {
                            tx.send(path.into()).expect("Failed to send picked folder path");
                        }
                    });
                }
                ui.separator();

                if let Some(current_dir_path) = self.current_dir.clone() {
                    self.display_dir_tree(ui, &current_dir_path, 0);
                } else {
                    ui.label("Nenhum diretório aberto.");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Editor: {}", self.file_path.as_deref().unwrap_or("[No File Loaded]")));
            ui.separator();

            let total_lines = self.editor_content.len_lines();

            egui::ScrollArea::vertical().show_rows(ui, LINE_HEIGHT, total_lines, |ui_scroll_area, row_range| { // Renomeado para ui_scroll_area
                ui_scroll_area.horizontal(|ui_horizontal| {
                    ui_horizontal.vertical(|ui_vertical_numbers| {
                        ui_vertical_numbers.set_width(LINE_NUMBER_GUTTER_WIDTH);
                        ui_vertical_numbers.spacing_mut().item_spacing.y = 0.0;

                        for i in row_range.start..row_range.end {
                            ui_vertical_numbers.monospace(format!("{:>4}", i + 1));
                        }
                    });

                    // <--- CORREÇÃO AQUI: Capturar available_width ANTES de iniciar a closure vertical
                    let content_panel_available_width = ui_horizontal.available_width();

                    ui_horizontal.vertical(|ui_vertical_content| {
                        // Usar o valor capturado
                        ui_vertical_content.set_width(content_panel_available_width);
                        ui_vertical_content.spacing_mut().item_spacing.y = 0.0;

                        for line_ropey in self.editor_content.lines_at(row_range.start).take(row_range.len()) {
                            let line_str = line_ropey.as_str().unwrap_or("");
                            let trimmed_line = line_str.trim_end_matches('\n').trim_end_matches('\r');
                            ui_vertical_content.monospace(trimmed_line);
                        }
                    });
                });
            });
        });
    }
}

impl MyApp {
    fn display_dir_tree(&mut self, ui: &mut egui::Ui, path: &PathBuf, indent_level: usize) {
        let is_dir_expanded = *self.expanded_dirs.entry(path.clone()).or_insert(false);
        let indent = indent_level as f32 * 15.0;

        if path.is_dir() {
            ui.horizontal(|ui_dir_entry| {
                ui_dir_entry.add_space(indent);
                let toggle_icon = if is_dir_expanded { "▼" } else { "►" };
                if ui_dir_entry.button(toggle_icon).clicked() {
                    *self.expanded_dirs.entry(path.clone()).or_insert(false) = !is_dir_expanded;
                }
                ui_dir_entry.label(format!("📁 {}", path.file_name().unwrap_or_default().to_string_lossy()));
            });

            if is_dir_expanded {
                for entry in WalkDir::new(path).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
                    let entry_path = entry.path().to_path_buf();
                    if entry_path.is_dir() {
                        self.display_dir_tree(ui, &entry_path, indent_level + 1);
                    } else if entry_path.is_file() {
                        ui.horizontal(|ui_file_entry| {
                            ui_file_entry.add_space(indent + 30.0);
                            if ui_file_entry.button(format!("📄 {}", entry_path.file_name().unwrap_or_default().to_string_lossy())).clicked() {
                                match load_file_into_rope(&entry_path) {
                                    Ok(rope) => {
                                        self.editor_content = rope;
                                        self.file_path = Some(entry_path.to_string_lossy().into_owned());
                                        eprintln!("Arquivo '{}' carregado com sucesso!", entry_path.display());
                                    },
                                    Err(e) => {
                                        eprintln!("Erro ao carregar o arquivo '{}': {}", entry_path.display(), e);
                                    }
                                }
                            }
                        });
                    }
                }
            }
        }
    }
}

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