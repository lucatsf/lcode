// src/ui/app.rs

use eframe::egui;
use ropey::Rope;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use rfd::AsyncFileDialog;
use pollster;

// Constantes de layout (melhor definidas aqui ou em um módulo de config)
const LINE_HEIGHT: f32 = 16.0;
const LINE_NUMBER_GUTTER_WIDTH: f32 = 60.0;
const SIDE_PANEL_WIDTH: f32 = 200.0;

/// Representa um item do sistema de arquivos (arquivo ou diretório).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileSystemItem {
    File(PathBuf),
    Directory(PathBuf),
}

/// Struct para representar um arquivo aberto no editor (uma aba).
#[derive(Debug)]
pub struct EditorTab {
    pub path: PathBuf,
    pub content: Rope,
    pub is_modified: bool,
}

impl EditorTab {
    /// Cria uma nova aba do editor.
    pub fn new(path: PathBuf, content: Rope) -> Self {
        Self {
            path,
            content,
            is_modified: false,
        }
    }

    /// Retorna o nome do arquivo, com um asterisco se modificado.
    pub fn name(&self) -> String {
        let mut name = self.path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        if self.is_modified {
            name.push('*');
        }
        name
    }
}

/// A struct principal da aplicação Egui.
pub struct MyApp {
    pub current_dir: Option<PathBuf>,
    pub expanded_dirs: HashMap<PathBuf, bool>,
    pub picked_folder_tx: std::sync::mpsc::Sender<PathBuf>,
    pub picked_folder_rx: std::sync::mpsc::Receiver<PathBuf>,

    pub open_tabs: Vec<EditorTab>,
    pub selected_tab_idx: Option<usize>,
}

impl Default for MyApp {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        // O texto inicial pode ser definido aqui, ou o editor pode começar vazio.
        let initial_text = "Hello, lcode!\n\nEste é o nosso editor de código minimalista.\n\nClique em 'Abrir Diretório' para começar.\n".to_string();


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
                    // Chama a função display_dir_tree que agora é um método de MyApp
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
                return;
            }

            // Painel de Abas na parte superior do CentralPanel
            egui::TopBottomPanel::top("tabs_panel").show_inside(ui, |ui_tabs| {
                ui_tabs.horizontal(|ui_horizontal_tabs| {
                    egui::ScrollArea::horizontal().show(ui_horizontal_tabs, |ui_scroll_tabs| {
                        ui_scroll_tabs.spacing_mut().item_spacing.x = 5.0;

                        let mut tab_to_close: Option<usize> = None;
                        let mut tab_to_select: Option<usize> = None;

                        for (i, tab) in self.open_tabs.iter().enumerate() {
                            let is_selected = self.selected_tab_idx == Some(i);
                            let response = ui_scroll_tabs.selectable_value(&mut self.selected_tab_idx, Some(i), tab.name());

                            if response.clicked() {
                                tab_to_select = Some(i);
                            }

                            let close_button_response = ui_scroll_tabs.add(egui::Button::new("x").small());
                            if close_button_response.clicked() {
                                tab_to_close = Some(i);
                            }
                        }

                        if let Some(idx) = tab_to_select {
                            self.selected_tab_idx = Some(idx);
                        }

                        if let Some(idx_to_close) = tab_to_close {
                            self.open_tabs.remove(idx_to_close);
                            if self.open_tabs.is_empty() {
                                self.selected_tab_idx = None;
                            } else if let Some(selected_idx) = self.selected_tab_idx {
                                if idx_to_close < selected_idx {
                                    self.selected_tab_idx = Some(selected_idx - 1);
                                } else if idx_to_close == selected_idx {
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

                                for line_ropey in current_tab.content.lines_at(row_range.start).take(row_range.len()) {
                                    let line_str = line_ropey.as_str().unwrap_or("");
                                    let trimmed_line = line_str.trim_end_matches('\n').trim_end_matches('\r');
                                    ui_vertical_content.monospace(trimmed_line);
                                }
                            });
                        });
                    });
                } else {
                    self.selected_tab_idx = None;
                }
            }
        });
    }
}