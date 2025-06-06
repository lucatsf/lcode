// src/ui/editor_ui.rs

use eframe::egui;
use ropey::Rope;
// Allow unused imports for Cursor and Selection as they are primarily used as type definitions
#[allow(unused_imports)]
use crate::core::editor::{Cursor, TextEditor, Selection};
use crate::syntax_highlighting::highlighter::SyntaxHighlighter;
use egui::text::LayoutJob;
use std::path::PathBuf;
use std::ops::Deref; // NOVO: Importar Deref


// Considere mover essas constantes para um módulo de configuração global
// (src/utils/constants.rs, por exemplo) para que possam ser usadas em toda a aplicação.
const LINE_HEIGHT: f32 = 16.0; // Altura da linha em pixels
const LINE_NUMBER_GUTTER_WIDTH: f32 = 60.0; // Largura da área dos números de linha


pub struct EditorPanel<'a> {
    pub content: &'a mut Rope,
    pub editor_state: &'a mut TextEditor,
    pub path: &'a PathBuf,
    pub highlighter: &'a SyntaxHighlighter,
    pub is_modified: &'a mut bool, // Para atualizar o estado 'modificado' da aba
}

impl<'a> EditorPanel<'a> {
    pub fn new(
        content: &'a mut Rope,
        editor_state: &'a mut TextEditor,
        path: &'a PathBuf,
        highlighter: &'a SyntaxHighlighter,
        is_modified: &'a mut bool,
    ) -> Self {
        Self {
            content,
            editor_state,
            path,
            highlighter,
            is_modified,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let text_style = egui::TextStyle::Monospace;
        let row_height = ui.text_style_height(&text_style);
        let total_lines = self.content.len_lines();

        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt("editor_scroll_area");

        // Sincroniza a posição de rolagem do editor_state com o ScrollArea do egui
        scroll_area = scroll_area.scroll_offset(self.editor_state.scroll_offset);

        let scroll_response = scroll_area.show_rows(ui, row_height, total_lines, |ui_scroll_area, row_range| { // Renomeado ui para ui_scroll_area
            ui_scroll_area.horizontal(|ui_horizontal| {
                // Gutter para números de linha
                ui_horizontal.vertical(|ui_vertical_numbers| {
                    ui_vertical_numbers.set_width(LINE_NUMBER_GUTTER_WIDTH);
                    ui_vertical_numbers.spacing_mut().item_spacing.y = 0.0;
                    ui_vertical_numbers.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                    for i in row_range.start..row_range.end {
                        ui_vertical_numbers.monospace(format!("{:>4}", i + 1));
                    }
                });

                // Painel de texto do editor principal
                ui_horizontal.add_space(ui_horizontal.available_width() * 0.01);
                // Renomeado para _editor_content_response para suprimir o warning de variável não usada
                let _editor_content_response = ui_horizontal.vertical(|ui_editor_content| {
                    ui_editor_content.set_width(ui_editor_content.available_width());
                    ui_editor_content.spacing_mut().item_spacing.y = 0.0;

                    // Renderização das linhas do editor
                    for line_idx in row_range.start..row_range.end {
                        let line_content = self.content.line(line_idx);
                        let line_str = line_content.as_str().unwrap_or("");
                        
                        let mut job = LayoutJob::default();
                        job.halign = egui::Align::LEFT;

                        // Realce de sintaxe
                        let highlighted_chunks = self.highlighter.highlight_line(line_str, self.path);
                        for (style, text) in highlighted_chunks {
                            let egui_color = SyntaxHighlighter::syntect_color_to_egui_color(style.foreground);
                            job.append(
                                text,
                                0.0, // Indentação
                                egui::TextFormat {
                                    font_id: egui::FontId::monospace(row_height * 0.9),
                                    color: egui_color,
                                    ..Default::default()
                                },
                            );
                        }
                        
                        // Obter o galley (informações de layout da linha) para desenhar o cursor/seleção
                        let galley = ui_editor_content.fonts(|f| f.layout_job(job));
                        
                        let line_response = ui_editor_content.label(galley.clone()); // Renderiza a linha

                        // Desenhar seleção de texto para esta linha (se houver)
                        self.draw_selection_on_line(ui_editor_content, line_idx, &galley, &line_response.rect);
                    }

                    // Lógica para capturar eventos de teclado e mouse e desenhar o cursor/seleção
                    // MOVIDO PARA DENTRO DESTA CLOSURE para que ui_editor_content esteja em escopo
                    self.handle_input_and_draw_cursor(ui_editor_content, &ui_editor_content.response(), row_height);
                }).response; // Captura a Response da área do conteúdo do editor (editor_content_response já está disponível)

                // Este call foi movido para dentro da closure ui_horizontal.vertical acima.
                // self.handle_input_and_draw_cursor(ui_horizontal.ctx(), &editor_content_response, row_height);
            });
        });

        // Atualiza o scroll_offset do editor_state com a nova posição de rolagem
        self.editor_state.scroll_offset = scroll_response.state.offset;
    }

    // Método para lidar com input e desenhar cursor/seleção
    // O primeiro parâmetro agora é 'ui: &mut egui::Ui'
    fn handle_input_and_draw_cursor(&mut self, ui: &mut egui::Ui, editor_area_response: &egui::Response, row_height: f32) {
        let ctx = ui.ctx(); // Obtém o contexto do ui
        // 1. Foco
        if editor_area_response.clicked() {
            editor_area_response.request_focus();
        }

        // 2. Input de Teclado (digitação de caracteres)
        if editor_area_response.has_focus() {
            ctx.input(|i| {
                for event in &i.events {
                    match event {
                        egui::Event::Text(text) => {
                            // Ignore paste events here, handle with Ctrl+V.
                            // Also ignore Enter key as it's handled by Key event.
                            if !(i.modifiers.command || i.modifiers.ctrl) && text != "\n" {
                                for ch in text.chars() {
                                    self.editor_state.insert_char(self.content, ch);
                                    *self.is_modified = true;
                                }
                                ctx.request_repaint();
                            }
                        },
                        egui::Event::Key { key, pressed, modifiers, .. } => {
                            if *pressed {
                                let mut handled = true;
                                if modifiers.command || modifiers.ctrl { // command on Mac, ctrl on Linux/Win
                                    match key {
                                        egui::Key::Z => {
                                            if modifiers.shift { // Ctrl+Shift+Z for Redo
                                                if self.editor_state.redo(self.content) {
                                                    *self.is_modified = true;
                                                }
                                            } else { // Ctrl+Z for Undo
                                                if self.editor_state.undo(self.content) {
                                                    *self.is_modified = true;
                                                }
                                            }
                                        },
                                        egui::Key::C => { // Ctrl+C: Copy
                                            if let Some(selection) = self.editor_state.selection {
                                                let normalized = selection.normalized();
                                                let start_char_idx = self.content.line_to_char(normalized.start.line) + normalized.start.char_idx;
                                                let end_char_idx = self.content.line_to_char(normalized.end.line) + normalized.end.char_idx;
                                                let selected_text = self.content.slice(start_char_idx..end_char_idx).to_string();
                                                ctx.copy_text(selected_text); // Use ctx.copy_text
                                            }
                                        },
                                        egui::Key::X => { // Ctrl+X: Cut
                                            if let Some(selection) = self.editor_state.selection {
                                                let normalized = selection.normalized();
                                                let start_char_idx = self.content.line_to_char(normalized.start.line) + normalized.start.char_idx;
                                                let end_char_idx = self.content.line_to_char(normalized.end.line) + normalized.end.char_idx;
                                                let selected_text = self.content.slice(start_char_idx..end_char_idx).to_string();
                                                ctx.copy_text(selected_text); // Use ctx.copy_text
                                                self.editor_state.delete_selected_text(self.content);
                                                *self.is_modified = true;
                                            }
                                        },
                                        egui::Key::V => { // Ctrl+V: Paste
                                            // Corrected: Get pasted text from system clipboard via raw events
                                            if let Some(pasted_text) = i.raw.events.iter().filter_map(|event| {
                                                if let egui::Event::Paste(s) = event { Some(s.clone()) } else { None }
                                            }).last() {
                                                self.editor_state.insert_text(self.content, &pasted_text);
                                                *self.is_modified = true;
                                            }
                                        },
                                        _ => handled = false,
                                    }
                                } else { // Keys without Ctrl/Command
                                    match key {
                                        egui::Key::ArrowLeft => {
                                            self.editor_state.move_cursor_left(self.content);
                                            // Handle shift for selection
                                            if modifiers.shift { self.editor_state.extend_selection(); } else { self.editor_state.clear_selection(); }
                                        },
                                        egui::Key::ArrowRight => {
                                            self.editor_state.move_cursor_right(self.content);
                                            if modifiers.shift { self.editor_state.extend_selection(); } else { self.editor_state.clear_selection(); }
                                        },
                                        egui::Key::ArrowUp => {
                                            self.editor_state.move_cursor_up(self.content);
                                            if modifiers.shift { self.editor_state.extend_selection(); } else { self.editor_state.clear_selection(); }
                                        },
                                        egui::Key::ArrowDown => {
                                            self.editor_state.move_cursor_down(self.content);
                                            if modifiers.shift { self.editor_state.extend_selection(); } else { self.editor_state.clear_selection(); }
                                        },
                                        egui::Key::Backspace => {
                                            self.editor_state.delete_char_before_cursor(self.content);
                                            *self.is_modified = true;
                                        },
                                        egui::Key::Delete => {
                                            self.editor_state.delete_char_after_cursor(self.content);
                                            *self.is_modified = true;
                                        },
                                        egui::Key::Enter => {
                                            self.editor_state.new_line(self.content);
                                            *self.is_modified = true;
                                        },
                                        _ => handled = false,
                                    }
                                }
                                if handled {
                                    ctx.request_repaint(); // Repaint if input was handled
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });
        }
        
        // 3. Renderizar Cursor
        let painter = ui.painter(); // Usa o painter do ui passado para o método
        let editor_rect = editor_area_response.rect;

        // Calcular a posição de pixel do cursor
        let font_id = egui::FontId::monospace(row_height * 0.9);
        
        // Para calcular a posição X do cursor, precisamos do Galley da linha atual.
        // Em um editor real, você teria um cache de galleys.
        let cursor_line_content = self.content.line(self.editor_state.cursor.line).to_string();
        let cursor_line_galley = ui.fonts(|f| f.layout_job(egui::text::LayoutJob::simple(
            cursor_line_content,
            font_id.clone(),
            ui.style().visuals.text_color(), // Usar cor padrão para layout de cálculo
            ui.available_width() // Adicionado wrap_width para LayoutJob::simple
        )));

        // NOVO: Explicitamente obter a referência ao Galley do Arc e acessar os x_offsets
        let galley_ref_cursor: &egui::Galley = cursor_line_galley.deref();
        // Acessa a posição X do caractere usando glyphs e pos
        let cursor_x_offset_in_line = galley_ref_cursor.rows.get(0)
            .and_then(|row| row.glyphs.get(self.editor_state.cursor.char_idx))
            .map(|glyph_info| glyph_info.pos.x) // Agora está correto, usando .pos.x
            .unwrap_or_else(|| {
                // Fallback: se o glyph não for encontrado (ex: cursor no final da linha),
                // use a largura total da linha (se a linha não estiver vazia)
                let last_glyph_x = galley_ref_cursor.rows.get(0)
                    .and_then(|row| row.glyphs.last())
                    .map(|glyph_info| glyph_info.pos.x + glyph_info.advance_width) // CORRIGIDO: usando advance_width
                    .unwrap_or_default(); // 0.0 para linha vazia
                last_glyph_x
            });

        let cursor_x = editor_rect.left() + cursor_x_offset_in_line;
        let cursor_y_relative_to_scroll = self.editor_state.cursor.line as f32 * row_height;
        let cursor_y_on_screen = editor_rect.top() + cursor_y_relative_to_scroll - self.editor_state.scroll_offset.y;

        // Desenhar o cursor (se a área do editor tiver foco e o cursor estiver visível)
        if editor_area_response.has_focus() {
            let cursor_color = ui.style().visuals.text_color(); // ui.style() agora acessível
            let cursor_width = 2.0;
            let cursor_height = row_height;
            
            let cursor_visual_rect = egui::Rect::from_min_size(
                egui::pos2(cursor_x, cursor_y_on_screen),
                egui::vec2(cursor_width, cursor_height)
            );

            // Desenha o cursor apenas se ele estiver dentro da área visível do editor
            if editor_rect.intersects(cursor_visual_rect) {
                painter.rect_filled(cursor_visual_rect, 0.0, cursor_color);
            }
            ctx.request_repaint(); // Garante que o cursor pisque
        }
    }

    // Desenhar a seleção de texto para uma linha específica
    fn draw_selection_on_line(&self, ui: &mut egui::Ui, line_idx: usize, galley: &std::sync::Arc<egui::Galley>, line_rect: &egui::Rect) {
        if let Some(selection) = self.editor_state.selection {
            let normalized_selection = selection.normalized();

            // Verificar se a seleção está nesta linha
            let selection_starts_on_this_line = normalized_selection.start.line == line_idx;
            let selection_ends_on_this_line = normalized_selection.end.line == line_idx;
            let selection_spans_this_line = normalized_selection.start.line < line_idx && normalized_selection.end.line > line_idx;

            if selection_starts_on_this_line || selection_ends_on_this_line || selection_spans_this_line {
                let start_char_in_line = if selection_starts_on_this_line {
                    normalized_selection.start.char_idx
                } else {
                    0 // Começa do início da linha se a seleção vem de cima
                };

                let end_char_in_line = if selection_ends_on_this_line {
                    normalized_selection.end.char_idx
                } else {
                    self.content.line(line_idx).len_chars() // Vai até o fim da linha se a seleção continua abaixo
                };

                // Calcular a região selecionada em pixels usando glyphs e pos
                let galley_ref_selection: &egui::Galley = galley.deref();

                let x_start = galley_ref_selection.rows.get(0)
                    .and_then(|row| row.glyphs.get(start_char_in_line))
                    .map(|glyph_info| glyph_info.pos.x)
                    .unwrap_or_default(); // 0.0 se não encontrar

                let x_end = galley_ref_selection.rows.get(0)
                    .and_then(|row| {
                        // Se o end_char_in_line for o índice do próximo caractere após o último da linha,
                        // queremos a largura total da linha.
                        if end_char_in_line == row.glyphs.len() {
                            Some(row.rect.width()) // Largura total da linha
                        } else {
                            // Se estiver apontando para um caractere dentro da linha, use a posição X + largura desse caractere
                            row.glyphs.get(end_char_in_line)
                                .map(|glyph_info| glyph_info.pos.x) // Posição do início do próximo caractere
                        }
                    })
                    .unwrap_or_default(); // 0.0 se não encontrar

                let selection_rect = egui::Rect::from_min_max(
                    egui::pos2(x_start, line_rect.top()),
                    egui::pos2(x_end, line_rect.bottom()),
                );
                
                // Ajustar a posição do retângulo de seleção para o espaço do editor_content
                let mut adjusted_selection_rect = selection_rect;
                adjusted_selection_rect = adjusted_selection_rect.translate(line_rect.left_top().to_vec2());


                let selection_color = ui.style().visuals.selection.bg_fill;
                ui.painter().rect_filled(adjusted_selection_rect, 0.0, selection_color);
            }
        }
    }
}