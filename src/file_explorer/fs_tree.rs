// src/file_explorer/fs_tree.rs

use eframe::egui;
use walkdir::WalkDir;
use std::path::PathBuf;

use crate::ui::app::{EditorTab, MyApp};
use crate::core::file_handler;

use egui_phosphor::variants::{fill, regular}; // Módulos de variantes

// Função auxiliar para obter o ícone Phosphor com base na extensão do arquivo
fn get_file_icon(path: &PathBuf) -> &'static str {
    if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
        match extension.to_lowercase().as_str() {
            // Exemplos de mapeamento para ícones específicos de linguagem
            "js" => regular::FILE_JS, // Ou um ícone mais específico se disponível, como 'regular::FILE_JS'
            "jsx" => regular::FILE_JSX,
            "ts" => regular::FILE_TS,
            "tsx" => regular::FILE_TSX,
            "json" => regular::CODE_BLOCK,
            "py" => regular::FILE_PY,
            "sql" => regular::FILE_SQL,
            "rs" => regular::FILE_RS,
            "md" => regular::FILE_TEXT,
            "css" => regular::FILE_CSS,
            "html" | "htm" => regular::FILE_HTML,
            "c" => regular::FILE_C,
            "cpp" => regular::FILE_CPP,
            "txt" => regular::FILE_TEXT,
            // Adicione mais casos conforme necessário para outras linguagens ou tipos de arquivo
            _ => regular::FILE, // Ícone de arquivo genérico para extensões não mapeadas
        }
    } else {
        regular::FILE // Ícone de arquivo genérico se não houver extensão
    }
}

impl MyApp {
    pub fn display_dir_tree(&mut self, ui: &mut egui::Ui, path: &PathBuf, indent_level: usize) {
        let is_dir_expanded = *self.expanded_dirs.entry(path.clone()).or_insert(false);
        let indent = indent_level as f32 * 15.0;

        if path.is_dir() {
            ui.horizontal(|ui_dir_entry| {
                ui_dir_entry.add_space(indent);
                let toggle_icon = if is_dir_expanded {
                    fill::CARET_DOWN
                } else {
                    fill::CARET_RIGHT
                };

                if ui_dir_entry.add(egui::Button::new(toggle_icon).small()).clicked() {
                    *self.expanded_dirs.entry(path.clone()).or_insert(false) = !is_dir_expanded;
                }
                ui_dir_entry.label(format!("{} {}", regular::FOLDER_SIMPLE, path.file_name().unwrap_or_default().to_string_lossy()));
            });

            if is_dir_expanded {
                for entry in WalkDir::new(path).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
                    let entry_path = entry.path().to_path_buf();
                    if entry_path.is_dir() {
                        self.display_dir_tree(ui, &entry_path, indent_level + 1);
                    } else if entry_path.is_file() {
                        ui.horizontal(|ui_file_entry| {
                            ui_file_entry.add_space(indent + 30.0);
                            // Use a nova função auxiliar para obter o ícone correto
                            let file_icon = get_file_icon(&entry_path);
                            if ui_file_entry.button(format!("{} {}", file_icon, entry_path.file_name().unwrap_or_default().to_string_lossy())).clicked() {
                                if let Some(idx) = self.open_tabs.iter().position(|tab| tab.path == entry_path) {
                                    self.selected_tab_idx = Some(idx);
                                    eprintln!("Arquivo '{}' já aberto, focando na aba existente.", entry_path.display());
                                } else {
                                    match file_handler::load_file_into_rope(&entry_path) {
                                        Ok(rope) => {
                                            let new_tab = EditorTab::new(entry_path.clone(), rope);
                                            self.open_tabs.push(new_tab);
                                            self.selected_tab_idx = Some(self.open_tabs.len() - 1);
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