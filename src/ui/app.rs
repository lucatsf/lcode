// src/ui/app.rs

use eframe::egui;
use ropey::Rope;
use std::collections::HashMap;
use std::path::PathBuf;
use rfd::AsyncFileDialog;
use pollster;

// Importar a função de salvamento do nosso módulo core
use crate::core::file_handler;
use crate::syntax_highlighting::highlighter::SyntaxHighlighter;
use egui::text::LayoutJob; // Importar LayoutJob


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
    // pub text_edit_content: String, // Este campo será eventualmente removido para otimização de memória
}

impl EditorTab {
    /// Cria uma nova aba do editor.
    pub fn new(path: PathBuf, content: Rope) -> Self {
        Self {
            path,
            content,
            is_modified: false,
            // text_edit_content: content.to_string(), // Inicialmente, para compatibilidade com TextEdit
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
    pub show_unsaved_changes_dialog: bool,
    pub dialog_tab_idx_to_close: Option<usize>,
    pub highlighter: SyntaxHighlighter,
    pub editor_scroll_offset: egui::Vec2, // Para controlar o scroll do editor manualmente
}

impl Default for MyApp {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let _initial_text = "Hello, lcode!\n\nEste é o nosso editor de código minimalista.\n\nClique em 'Abrir Diretório' para começar.\n".to_string();


        Self {
            current_dir: None,
            expanded_dirs: HashMap::new(),
            picked_folder_tx: tx,
            picked_folder_rx: rx,
            open_tabs: Vec::new(),
            selected_tab_idx: None,
            show_unsaved_changes_dialog: false,
            dialog_tab_idx_to_close: None,
            highlighter: SyntaxHighlighter::new(),
            editor_scroll_offset: egui::Vec2::ZERO,
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
                    self.display_dir_tree(ui, &current_dir_path, 0);
                } else {
                    ui.label("Nenhum diretório aberto.");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.open_tabs.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Nenhum arquivo aberto. Selecione um arquivo no explorador.");
                });
                return;
            }

            egui::TopBottomPanel::top("tabs_panel").show_inside(ui, |ui_tabs| {
                ui_tabs.horizontal(|ui_horizontal_tabs| {
                    egui::ScrollArea::horizontal().show(ui_horizontal_tabs, |ui_scroll_tabs| {
                        ui_scroll_tabs.spacing_mut().item_spacing.x = 5.0;

                        let mut tab_to_close_directly: Option<usize> = None;
                        let mut tab_to_select: Option<usize> = None;

                        for (i, tab) in self.open_tabs.iter().enumerate() {
                            let _is_selected = self.selected_tab_idx == Some(i);
                            let response = ui_scroll_tabs.selectable_value(&mut self.selected_tab_idx, Some(i), tab.name());

                            if response.clicked() {
                                tab_to_select = Some(i);
                            }

                            let close_button_response = ui_scroll_tabs.add(egui::Button::new("x").small());
                            if close_button_response.clicked() {
                                if tab.is_modified {
                                    self.show_unsaved_changes_dialog = true;
                                    self.dialog_tab_idx_to_close = Some(i);
                                    eprintln!("Tentando fechar aba modificada. Mostrando diálogo.");
                                } else {
                                    tab_to_close_directly = Some(i);
                                }
                            }
                        }

                        if let Some(idx) = tab_to_select {
                            self.selected_tab_idx = Some(idx);
                        }

                        if let Some(idx_to_close) = tab_to_close_directly {
                            self.close_tab(idx_to_close);
                        }
                    });
                });
            });

            // Conteúdo do Editor para a aba selecionada
            if let Some(selected_idx) = self.selected_tab_idx {
                if let Some(current_tab) = self.open_tabs.get_mut(selected_idx) {
                    ui.heading(format!("Editor: {}", current_tab.name()));
                    ui.separator();

                    let text_style = egui::TextStyle::Monospace;
                    let row_height = ui.text_style_height(&text_style);

                    // FR.2.5: Números de Linha
                    let total_lines = current_tab.content.len_lines();

                    let mut layouter = |ui: &egui::Ui, s: &str, wrap_width: f32| {
                        let mut job = LayoutJob::default();
                        // Remover job.wrap_width = wrap_width; // Já removido

                        job.halign = egui::Align::LEFT;

                        let highlighted_chunks = self.highlighter.highlight_line(s, &current_tab.path);
                        for (style, text) in highlighted_chunks {
                            let egui_color = SyntaxHighlighter::syntect_color_to_egui_color(style.foreground);
                            job.append(
                                text,
                                0.0, // Indentação
                                egui::TextFormat {
                                    font_id: egui::FontId::monospace(row_height * 0.9), // Ajustar o tamanho da fonte
                                    color: egui_color,
                                    // Adicionar outros estilos se necessário (negrito, itálico)
                                    ..Default::default()
                                },
                            );
                        }
                        ui.fonts(|f| f.layout_job(job))
                    };

                    egui::ScrollArea::vertical()
                        .id_source("editor_scroll_area")
                        .show_rows(ui, row_height, total_lines, |ui, row_range| {
                            ui.horizontal(|ui_horizontal| {
                                // Gutter para números de linha
                                ui_horizontal.vertical(|ui_vertical_numbers| {
                                    ui_vertical_numbers.set_width(LINE_NUMBER_GUTTER_WIDTH);
                                    ui_vertical_numbers.spacing_mut().item_spacing.y = 0.0;
                                    ui_vertical_numbers.style_mut().wrap = Some(false);

                                    for i in row_range.start..row_range.end {
                                        ui_vertical_numbers.monospace(format!("{:>4}", i + 1));
                                    }
                                });

                                // Painel de texto do editor
                                ui_horizontal.add_space(ui_horizontal.available_width() * 0.01); // Pequeno espaçamento
                                ui_horizontal.vertical(|ui_editor_content| {
                                    ui_editor_content.set_width(ui_editor_content.available_width());
                                    ui_editor_content.spacing_mut().item_spacing.y = 0.0;

                                    // Iterar e renderizar linhas visíveis diretamente do Rope
                                    for line_idx in row_range.start..row_range.end {
                                        if let Some(line) = current_tab.content.get_line(line_idx) {
                                            // Usar o layouter para obter o LayoutJob com realce de sintaxe
                                            let job = layouter(ui_editor_content, line.as_str().unwrap_or(""), ui_editor_content.available_width());
                                            ui_editor_content.label(job);
                                        }
                                    }
                                    // Com esta abordagem, o TextEdit não é usado para exibição nem para edição.
                                    // A detecção de modificação e salvamento de arquivos precisará de uma nova lógica de entrada do usuário.
                                });
                            });
                        });


                    // FR.2.3.2: Salvar arquivos usando Ctrl+S
                    if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
                        eprintln!("Ctrl+S pressionado.");
                        if current_tab.is_modified {
                            self.save_current_tab(ctx);
                        } else {
                            eprintln!("Arquivo não modificado, não há o que salvar.");
                        }
                    }

                } else {
                    self.selected_tab_idx = None;
                }
            }
        });

        // Diálogo de confirmação para alterações não salvas (FR.2.3.3)
        if self.show_unsaved_changes_dialog {
            self.draw_unsaved_changes_dialog(ctx);
        }
    }
}

// Métodos auxiliares para MyApp
impl MyApp {
    // Nova função para salvar a aba atualmente selecionada
    fn save_current_tab(&mut self, ctx: &egui::Context) {
        if let Some(selected_idx) = self.selected_tab_idx {
            if let Some(current_tab) = self.open_tabs.get_mut(selected_idx) {
                eprintln!("Salvando arquivo: {}", current_tab.path.display());
                match file_handler::save_rope_to_file(&current_tab.path, &current_tab.content) {
                    Ok(_) => {
                        current_tab.is_modified = false;
                        eprintln!("Arquivo salvo com sucesso!");
                        ctx.request_repaint(); // Força a UI a atualizar para remover o '*'
                    },
                    Err(e) => {
                        eprintln!("Erro ao salvar arquivo: {}", e);
                        // TODO: Exibir erro para o usuário na UI
                    }
                }
            }
        }
    }

    // Nova função para fechar uma aba pelo índice
    fn close_tab(&mut self, idx_to_close: usize) {
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

    // Nova função para desenhar o diálogo de alterações não salvas
    fn draw_unsaved_changes_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_unsaved_changes_dialog;
        egui::Window::new("Alterações Não Salvas")
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .show(ctx, |ui| {
                ui.label("Você tem alterações não salvas. Deseja salvar, descartar ou cancelar?");
                ui.add_space(10.0);

                ui.horizontal(|ui_buttons| {
                    if ui_buttons.button("Salvar").clicked() {
                        if let Some(idx) = self.dialog_tab_idx_to_close {
                            self.selected_tab_idx = Some(idx); // Seleciona a aba para salvá-la
                            self.save_current_tab(ctx);
                            self.close_tab(idx); // Fecha após salvar
                        }
                        open = false;
                    }
                    if ui_buttons.button("Descartar").clicked() {
                        if let Some(idx) = self.dialog_tab_idx_to_close {
                            self.close_tab(idx); // Apenas fecha, descartando as alterações
                        }
                        open = false;
                    }
                    if ui_buttons.button("Cancelar").clicked() {
                        open = false; // Não faz nada, mantém a aba aberta
                    }
                });
            });
        self.show_unsaved_changes_dialog = open;
        if !open { // Se o diálogo foi fechado por qualquer ação, limpar o índice
            self.dialog_tab_idx_to_close = None;
        }
    }
}