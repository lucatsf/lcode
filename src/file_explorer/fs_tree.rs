// src/file_explorer/fs_tree.rs

use eframe::egui;
// Removidas as importações desnecessárias, pois HashMap é usado em MyApp
// e Path é usado indiretamente via PathBuf ou em file_handler.
// use std::collections::HashMap;
use walkdir::WalkDir;
use std::path::PathBuf; // PathBuf é o que você usa diretamente aqui

use crate::ui::app::{EditorTab, MyApp};
use crate::core::file_handler;

// As constantes do layout também podem ser movidas para um módulo de "configurações"
// ou mantidas no MyApp se forem específicas da UI. Por agora, vamos mantê-las aqui
// ou onde forem mais usadas.

/// Exibe a árvore de diretórios no painel lateral.
///
/// Esta função lida com a navegação de diretórios, expansão/retração
/// e a abertura de arquivos em novas abas do editor.
///
/// # Argumentos
///
/// * `ui` - O UI para desenhar os widgets.
/// * `path` - O caminho do diretório atual sendo exibido.
/// * `indent_level` - O nível de indentação para a exibição (para subdiretórios).
impl MyApp { // Colocamos a função como um método de MyApp
    pub fn display_dir_tree(&mut self, ui: &mut egui::Ui, path: &PathBuf, indent_level: usize) {
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
                                    match file_handler::load_file_into_rope(&entry_path) {
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