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
use crate::terminal::pty_integration::Terminal; // Apenas Terminal, não precisamos de TerminalOutput aqui
use egui::TextWrapMode; 
use crate::ui::editor_ui::EditorPanel;
use crate::core::editor::TextEditor;
use std::sync::Arc;


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
    pub editor_state: TextEditor,
    pub galley_cache: Vec<Option<Arc<egui::Galley>>>, // NOVO: Cache de galleys
    pub last_content_len: usize, // NOVO: Para detectar mudanças de tamanho do conteúdo
}

impl EditorTab {
    /// Cria uma nova aba do editor.
    pub fn new(path: PathBuf, content: Rope) -> Self {
        let initial_len = content.len_lines();
        Self {
            path,
            content,
            is_modified: false,
            editor_state: TextEditor::new(),
            galley_cache: vec![None; initial_len], // Inicializa o cache com o número de linhas
            last_content_len: initial_len, // Guarda o comprimento inicial
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
    pub terminal: Terminal, // Adicionar o terminal aqui
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
            terminal: Terminal::new(), // Inicializar o terminal
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

        // Botão para abrir/fechar o terminal (FR.3.1.1)
        egui::TopBottomPanel::bottom("terminal_panel_toggle").show(ctx, |ui| {
            ui.horizontal(|ui_horizontal| {
                if ui_horizontal.button("Abrir Terminal").clicked() {
                    self.terminal.is_open = !self.terminal.is_open;
                    if self.terminal.is_open {
                        self.terminal.start(self.current_dir.clone()); // Iniciar o terminal ao abrir (FR.3.2.2)
                    } else {
                        self.terminal.stop(); // Parar o terminal ao fechar
                    }
                }
            });
        });

        // O painel principal do editor/terminal
        egui::CentralPanel::default().show(ctx, |ui| {
            // Se o terminal estiver aberto, dividimos o espaço
            if self.terminal.is_open {
                egui::TopBottomPanel::bottom("integrated_terminal_panel")
                    .resizable(true) // FR.3.1.2
                    .min_height(50.0)
                    .show_inside(ui, |ui_terminal| {
                        self.terminal.ui(ui_terminal, self.current_dir.clone());
                    });
            }

            // O conteúdo do editor principal ocupa o espaço restante do CentralPanel
            if self.open_tabs.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Nenhum arquivo aberto. Selecione um arquivo no explorador.");
                });
                return;
            }

            // O TopBottomPanel para as abas já está correto
            egui::TopBottomPanel::top("tabs_panel").show_inside(ui, |ui_tabs| {
                ui_tabs.horizontal(|ui_horizontal_tabs| {
                    egui::ScrollArea::horizontal().show(ui_horizontal_tabs, |ui_scroll_tabs| {
                        ui_scroll_tabs.spacing_mut().item_spacing.x = 5.0;

                        let mut tab_to_close_directly: Option<usize> = None;
                        let mut tab_to_select: Option<usize> = None;

                        for (i, tab) in self.open_tabs.iter().enumerate() {
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

                    // NOVO: Criar e mostrar o EditorPanel
                    let mut editor_panel = EditorPanel::new(
                        &mut current_tab.content,
                        &mut current_tab.editor_state,
                        &current_tab.path,
                        &self.highlighter,
                        &mut current_tab.is_modified,
                        &mut current_tab.galley_cache, // NOVO
                        &mut current_tab.last_content_len, // NOVO
                    );
                    editor_panel.show(ui);

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

fn apply_string_diff_to_rope(
    rope: &mut Rope,
    old_s: &str,
    new_s: &str,
) {
    let mut common_prefix_len = 0;
    for (old_char, new_char) in old_s.chars().zip(new_s.chars()) {
        if old_char == new_char {
            common_prefix_len += old_char.len_utf8();
        } else {
            break;
        }
    }

    let mut common_suffix_len = 0;
    for (old_char, new_char) in old_s.chars().rev().zip(new_s.chars().rev()) {
        if old_char == new_char {
            common_suffix_len += old_char.len_utf8();
        } else {
            break;
        }
    }

    if common_prefix_len + common_suffix_len > old_s.len().min(new_s.len()) {
        return;
    }

    // Calcular os índices de byte da parte alterada no old_s
    let old_diff_start_byte = common_prefix_len;
    let old_diff_end_byte = old_s.len() - common_suffix_len;

    // Calcular os índices de byte da parte alterada no new_s
    let new_diff_start_byte = common_prefix_len;
    let new_diff_end_byte = new_s.len() - common_suffix_len;

    // Converter índices de byte para índices de caractere para o Rope
    let old_diff_start_char = rope.byte_to_char(old_diff_start_byte);
    let old_diff_end_char = rope.byte_to_char(old_diff_end_byte);

    // A parte que foi removida (se houver)
    // Garanta que old_diff_start_char <= old_diff_end_char para evitar overflow
    if old_diff_start_char < old_diff_end_char { // Modificado para evitar overflow
        rope.remove(old_diff_start_char..old_diff_end_char);
        eprintln!("Removidos {} chars a partir do byte {}", old_diff_end_char - old_diff_start_char, old_diff_start_char);
    }
    // else { // Debugging:
    //     eprintln!("Não removeu: start_char {} >= end_char {}", old_diff_start_char, old_diff_end_char);
    //     eprintln!("old_s len: {}, new_s len: {}", old_s.len(), new_s.len());
    //     eprintln!("common_prefix_len: {}, common_suffix_len: {}", common_prefix_len, common_suffix_len);
    //     eprintln!("old_diff_start_byte: {}, old_diff_end_byte: {}", old_diff_start_byte, old_diff_end_byte);
    // }


    // A parte que foi inserida (se houver)
    let chars_to_insert = &new_s[new_diff_start_byte..new_diff_end_byte];

    if !chars_to_insert.is_empty() {
        // A inserção ocorre no `old_diff_start_char` porque é onde a parte removida começaria
        // ou onde a nova parte deve ser inserida se nada foi removido mas algo foi adicionado.
        rope.insert(old_diff_start_char, chars_to_insert);
        eprintln!("Inseridos '{}' a partir do byte {}", chars_to_insert, old_diff_start_byte);
    }
}