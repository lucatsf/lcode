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

// Nova struct para representar um arquivo aberto (uma aba do editor)
#[derive(Debug)]
struct EditorTab {
    path: PathBuf,
    content: Rope,
    is_modified: bool,
}

impl EditorTab {
    fn new(path: PathBuf, content: Rope) -> Self {
        Self {
            path,
            content,
            is_modified: false, // Novo arquivo/recém-aberto não está modificado
        }
    }

    fn name(&self) -> String {
        let mut name = self.path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        if self.is_modified {
            name.push('*'); // Adiciona asterisco se modificado
        }
        name
    }
}

struct MyApp {
    // Campos antigos removidos: editor_content, file_path
    current_dir: Option<PathBuf>,
    expanded_dirs: HashMap<PathBuf, bool>,
    picked_folder_tx: std::sync::mpsc::Sender<PathBuf>,
    picked_folder_rx: std::sync::mpsc::Receiver<PathBuf>,

    // NOVOS campos para gerenciamento de abas
    open_tabs: Vec<EditorTab>,
    selected_tab_idx: Option<usize>, // Índice da aba atualmente selecionada
}

impl Default for MyApp {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let initial_text = "Hello, lcode!\n\nEste é o nosso editor de código minimalista.\n\nClique em 'Abrir Diretório' para começar.\n".to_string();

        // Removida a lógica de carregamento de test_large_file.txt do default
        // para que o editor comece vazio e incentive a abertura via explorador.
        // Se desejar ter um arquivo inicial, você pode adicionar uma aba aqui.

        Self {
            current_dir: None,
            expanded_dirs: HashMap::new(),
            picked_folder_tx: tx,
            picked_folder_rx: rx,
            open_tabs: Vec::new(),
            selected_tab_idx: None,
        }
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
                    // Passa o closure de `on_file_selected` para a função de exibição da árvore
                    self.display_dir_tree(ui, &current_dir_path, 0);
                } else {
                    ui.label("Nenhum diretório aberto.");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Se não houver abas abertas, mostra uma mensagem inicial
            if self.open_tabs.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Nenhum arquivo aberto. Selecione um arquivo no explorador.");
                });
                return; // Sai da função update para o painel central
            }

            // Painel de Abas na parte superior do CentralPanel
            egui::TopBottomPanel::top("tabs_panel").show_inside(ui, |ui_tabs| {
                ui_tabs.horizontal(|ui_horizontal_tabs| {
                    // Adiciona um ScrollArea horizontal para as abas, se muitas abas forem abertas
                    egui::ScrollArea::horizontal().show(ui_horizontal_tabs, |ui_scroll_tabs| {
                        ui_scroll_tabs.spacing_mut().item_spacing.x = 5.0; // Espaçamento entre as abas

                        let mut tab_to_close: Option<usize> = None;
                        let mut tab_to_select: Option<usize> = None;

                        for (i, tab) in self.open_tabs.iter().enumerate() {
                            let is_selected = self.selected_tab_idx == Some(i);
                            // Cria um botão de "aba"
                            let response = ui_scroll_tabs.selectable_value(&mut self.selected_tab_idx, Some(i), tab.name());

                            if response.clicked() {
                                tab_to_select = Some(i);
                            }

                            // Botão de fechar (x) para cada aba
                            let close_button_response = ui_scroll_tabs.add(egui::Button::new("x").small());
                            if close_button_response.clicked() {
                                tab_to_close = Some(i);
                            }
                        }

                        // Lida com a seleção da aba
                        if let Some(idx) = tab_to_select {
                            self.selected_tab_idx = Some(idx);
                        }

                        // Lida com o fechamento da aba
                        if let Some(idx_to_close) = tab_to_close {
                            self.open_tabs.remove(idx_to_close);
                            if self.open_tabs.is_empty() {
                                self.selected_tab_idx = None;
                            } else if let Some(selected_idx) = self.selected_tab_idx {
                                // Ajusta o índice selecionado se a aba fechada estava antes dele
                                if idx_to_close < selected_idx {
                                    self.selected_tab_idx = Some(selected_idx - 1);
                                } else if idx_to_close == selected_idx {
                                    // Se a aba selecionada foi fechada, seleciona a próxima ou a anterior
                                    self.selected_tab_idx = Some(idx_to_close.min(self.open_tabs.len().saturating_sub(1)));
                                }
                            }
                        }
                    });
                });
            });

            // Conteúdo do Editor para a aba selecionada
            if let Some(selected_idx) = self.selected_tab_idx {
                if let Some(current_tab) = self.open_tabs.get_mut(selected_idx) {
                    ui.heading(format!("Editor: {}", current_tab.name()));
                    ui.separator();

                    let total_lines = current_tab.content.len_lines();

                    egui::ScrollArea::vertical().show_rows(ui, LINE_HEIGHT, total_lines, |ui_scroll_area, row_range| {
                        ui_scroll_area.horizontal(|ui_horizontal| {
                            ui_horizontal.vertical(|ui_vertical_numbers| {
                                ui_vertical_numbers.set_width(LINE_NUMBER_GUTTER_WIDTH);
                                ui_vertical_numbers.spacing_mut().item_spacing.y = 0.0;

                                for i in row_range.start..row_range.end {
                                    ui_vertical_numbers.monospace(format!("{:>4}", i + 1));
                                }
                            });

                            let content_panel_available_width = ui_horizontal.available_width();

                            ui_horizontal.vertical(|ui_vertical_content| {
                                ui_vertical_content.set_width(content_panel_available_width);
                                ui_vertical_content.spacing_mut().item_spacing.y = 0.0;

                                // Aqui, adicionaremos a lógica de edição de texto na próxima fase.
                                // Por enquanto, apenas exibimos o texto.
                                for line_ropey in current_tab.content.lines_at(row_range.start).take(row_range.len()) {
                                    let line_str = line_ropey.as_str().unwrap_or("");
                                    let trimmed_line = line_str.trim_end_matches('\n').trim_end_matches('\r');
                                    ui_vertical_content.monospace(trimmed_line);
                                }
                            });
                        });
                    });
                } else {
                    // Caso o índice selecionado seja inválido por algum motivo
                    self.selected_tab_idx = None;
                }
            }
        });
    }
}

impl MyApp {
    // Modificado para lidar com a abertura de abas
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
                                // FR.1.3.2: Se o arquivo já estiver aberto, focar na aba existente
                                if let Some(idx) = self.open_tabs.iter().position(|tab| tab.path == entry_path) {
                                    self.selected_tab_idx = Some(idx);
                                    eprintln!("Arquivo '{}' já aberto, focando na aba existente.", entry_path.display());
                                } else {
                                    match load_file_into_rope(&entry_path) {
                                        Ok(rope) => {
                                            let new_tab = EditorTab::new(entry_path.clone(), rope);
                                            self.open_tabs.push(new_tab);
                                            self.selected_tab_idx = Some(self.open_tabs.len() - 1); // Seleciona a nova aba
                                            eprintln!("Arquivo '{}' carregado e nova aba criada.", entry_path.display());
                                        },
                                        Err(e) => {
                                            eprintln!("Erro ao carregar o arquivo '{}': {}", entry_path.display(), e);
                                        }
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