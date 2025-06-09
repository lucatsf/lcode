// src/ui/editor_ui.rs

use eframe::egui;
use ropey::Rope;
use crate::core::editor::{Cursor, TextEditor, Selection};
use crate::syntax_highlighting::highlighter::SyntaxHighlighter;
use egui::text::LayoutJob;
use egui::TextWrapMode;
use std::path::PathBuf;
use std::ops::Deref;
use std::sync::Arc;

const LINE_HEIGHT: f32 = 16.0;
const LINE_NUMBER_GUTTER_WIDTH: f32 = 60.0;

pub struct EditorPanel<'a> {
    pub content: &'a mut Rope,
    pub editor_state: &'a mut TextEditor,
    pub path: &'a PathBuf,
    pub highlighter: &'a SyntaxHighlighter,
    pub is_modified: &'a mut bool,
    pub galley_cache: &'a mut Vec<Option<Arc<egui::Galley>>>,
    pub last_content_len: &'a mut usize,
}

impl<'a> EditorPanel<'a> {
    pub fn new(
        content: &'a mut Rope,
        editor_state: &'a mut TextEditor,
        path: &'a PathBuf,
        highlighter: &'a SyntaxHighlighter,
        is_modified: &'a mut bool,
        galley_cache: &'a mut Vec<Option<Arc<egui::Galley>>>,
        last_content_len: &'a mut usize,
    ) -> Self {
        Self {
            content,
            editor_state,
            path,
            highlighter,
            is_modified,
            galley_cache,
            last_content_len,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let text_style = egui::TextStyle::Monospace;
        let row_height = ui.text_style_height(&text_style);
        let total_lines = self.content.len_lines();

        if total_lines != *self.last_content_len {
            self.galley_cache.resize_with(total_lines, || None);
            *self.last_content_len = total_lines;
        }

        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt("editor_scroll_area");

        scroll_area = scroll_area.scroll_offset(self.editor_state.scroll_offset);

        let scroll_response = scroll_area.show_rows(ui, row_height, total_lines, |ui_scroll_area, row_range| {
            ui_scroll_area.horizontal(|ui_horizontal| {
                ui_horizontal.vertical(|ui_vertical_numbers| {
                    ui_vertical_numbers.set_width(LINE_NUMBER_GUTTER_WIDTH);
                    ui_vertical_numbers.spacing_mut().item_spacing.y = 0.0;
                    ui_vertical_numbers.style_mut().wrap_mode = Some(TextWrapMode::Extend);

                    for i in row_range.start..row_range.end {
                        ui_vertical_numbers.monospace(format!("{:>4}", i + 1));
                    }
                });

                ui_horizontal.add_space(ui_horizontal.available_width() * 0.01);
                
                let editor_interaction_response = ui_horizontal.vertical(|ui_editor_content| {
                    ui_editor_content.set_width(ui_editor_content.available_width());
                    ui_editor_content.spacing_mut().item_spacing.y = 0.0;

                    for line_idx in row_range.start..row_range.end {
                        let galley_to_render = self.galley_cache[line_idx].clone().unwrap_or_else(|| {
                            let line_content = self.content.line(line_idx);
                            let line_str = line_content.as_str().unwrap_or("");
                            
                            let mut job = LayoutJob::default();
                            job.halign = egui::Align::LEFT;

                            let highlighted_chunks = self.highlighter.highlight_line(line_str, self.path);
                            for (style, text) in highlighted_chunks {
                                let egui_color = SyntaxHighlighter::syntect_color_to_egui_color(style.foreground);
                                job.append(
                                    text,
                                    0.0,
                                    egui::TextFormat {
                                        font_id: egui::FontId::monospace(row_height * 0.9),
                                        color: egui_color,
                                        ..Default::default()
                                    },
                                );
                            }
                            let new_galley = ui_editor_content.fonts(|f| f.layout_job(job));
                            self.galley_cache[line_idx] = Some(new_galley.clone());
                            new_galley
                        });
                        
                        let line_response = ui_editor_content.label(galley_to_render.clone());
                        self.draw_selection_on_line(ui_editor_content, line_idx, &galley_to_render, &line_response.rect);
                    }

                    let full_editor_rect = ui_editor_content.available_rect_before_wrap();
                    let id = ui_editor_content.id().with("full_editor_interaction_area");
                    ui_editor_content.interact(full_editor_rect, id, egui::Sense::click_and_drag())
                }).response;
                
                // Correção aqui: Passar ui_horizontal como o &mut Ui
                self.handle_input_and_draw_cursor(ui_horizontal, &editor_interaction_response, row_height);
            });
        });

        self.editor_state.scroll_offset = scroll_response.state.offset;
    }

    fn handle_input_and_draw_cursor(&mut self, ui: &mut egui::Ui, editor_area_response: &egui::Response, row_height: f32) {
        let ctx = ui.ctx();
        if editor_area_response.clicked() {
            eprintln!("Editor area clicked!");
            editor_area_response.request_focus();
        }

        if editor_area_response.has_focus() {
            eprintln!("Editor has focus. Processing input.");
            ctx.input(|i| {
                for event in &i.events {
                    match event {
                        egui::Event::Text(text) => {
                            eprintln!("Received text: '{}'", text);
                            if !(i.modifiers.command || i.modifiers.ctrl) && text != "\n" {
                                for ch in text.chars() {
                                    self.editor_state.insert_char(self.content, ch);
                                    *self.is_modified = true;
                                    self.invalidate_cache_from_line(self.editor_state.cursor.line);
                                }
                                ctx.request_repaint();
                            }
                        },
                        egui::Event::Key { key, pressed, modifiers, .. } => {
                            if *pressed {
                                eprintln!("Key pressed: {:?} (Modifiers: {:?})", key, modifiers);
                                let mut handled = true;
                                if modifiers.command || modifiers.ctrl {
                                    match key {
                                        egui::Key::Z => {
                                            if modifiers.shift {
                                                if self.editor_state.redo(self.content) {
                                                    *self.is_modified = true;
                                                    self.invalidate_cache_from_line(self.editor_state.cursor.line);
                                                }
                                            } else {
                                                if self.editor_state.undo(self.content) {
                                                    *self.is_modified = true;
                                                    self.invalidate_cache_from_line(self.editor_state.cursor.line);
                                                }
                                            }
                                        },
                                        egui::Key::C => {
                                            if let Some(selection) = self.editor_state.selection {
                                                let normalized = selection.normalized();
                                                let start_char_idx = self.content.line_to_char(normalized.start.line) + normalized.start.char_idx;
                                                let end_char_idx = self.content.line_to_char(normalized.end.line) + normalized.end.char_idx;
                                                let selected_text = self.content.slice(start_char_idx..end_char_idx).to_string();
                                                ctx.copy_text(selected_text);
                                            }
                                        },
                                        egui::Key::X => {
                                            if let Some(selection) = self.editor_state.selection {
                                                let normalized = selection.normalized();
                                                let start_char_idx = self.content.line_to_char(normalized.start.line) + normalized.start.char_idx;
                                                let end_char_idx = self.content.line_to_char(normalized.end.line) + normalized.end.char_idx;
                                                let selected_text = self.content.slice(start_char_idx..end_char_idx).to_string();
                                                ctx.copy_text(selected_text);
                                                self.editor_state.delete_selected_text(self.content);
                                                *self.is_modified = true;
                                                self.invalidate_cache_from_line(self.editor_state.cursor.line);
                                            }
                                        },
                                        egui::Key::V => {
                                            if let Some(pasted_text) = i.raw.events.iter().filter_map(|event| {
                                                if let egui::Event::Paste(s) = event { Some(s.clone()) } else { None }
                                            }).last() {
                                                self.editor_state.insert_text(self.content, &pasted_text);
                                                *self.is_modified = true;
                                                self.invalidate_cache_from_line(self.editor_state.cursor.line.saturating_sub(pasted_text.matches('\n').count()));
                                            }
                                        },
                                        _ => handled = false,
                                    }
                                } else {
                                    match key {
                                        egui::Key::ArrowLeft => {
                                            self.editor_state.move_cursor_left(self.content);
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
                                            self.invalidate_cache_from_line(self.editor_state.cursor.line);
                                        },
                                        egui::Key::Delete => {
                                            self.editor_state.delete_char_after_cursor(self.content);
                                            *self.is_modified = true;
                                            self.invalidate_cache_from_line(self.editor_state.cursor.line);
                                        },
                                        egui::Key::Enter => {
                                            self.editor_state.new_line(self.content);
                                            *self.is_modified = true;
                                            self.invalidate_cache_from_line(self.editor_state.cursor.line.saturating_sub(1));
                                        },
                                        _ => handled = false,
                                    }
                                }
                                if handled {
                                    ctx.request_repaint();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });
        } else {
            eprintln!("Editor does NOT have focus.");
        }
        
        let painter = ui.painter();
        let editor_rect = editor_area_response.rect;

        let font_id = egui::FontId::monospace(row_height * 0.9);
        
        let cursor_line_content = self.content.line(self.editor_state.cursor.line).to_string();
        let cursor_line_galley = ui.fonts(|f| f.layout_job(egui::text::LayoutJob::simple(
            cursor_line_content,
            font_id.clone(),
            ui.style().visuals.text_color(),
            ui.available_width()
        )));

        let galley_ref_cursor: &egui::Galley = &*cursor_line_galley;
        let cursor_x_offset_in_line = galley_ref_cursor.rows.get(0)
            .and_then(|row| {
                row.glyphs.get(self.editor_state.cursor.char_idx)
                    .map(|glyph_info| glyph_info.pos.x)
                    .or_else(|| {
                        row.glyphs.last()
                           .map(|glyph_info| glyph_info.pos.x + glyph_info.advance_width)
                    })
            })
            .unwrap_or(0.0);

        let cursor_x = editor_rect.left() + cursor_x_offset_in_line;
        let cursor_y_relative_to_scroll = self.editor_state.cursor.line as f32 * row_height;
        let cursor_y_on_screen = editor_rect.top() + cursor_y_relative_to_scroll - self.editor_state.scroll_offset.y;

        if editor_area_response.has_focus() {
            let cursor_color = ui.style().visuals.text_color();
            let cursor_width = 2.0;
            let cursor_height = row_height;
            
            let cursor_visual_rect = egui::Rect::from_min_size(
                egui::pos2(cursor_x, cursor_y_on_screen),
                egui::vec2(cursor_width, cursor_height)
            );

            if editor_rect.intersects(cursor_visual_rect) {
                painter.rect_filled(cursor_visual_rect, 0.0, cursor_color);
            }
            ctx.request_repaint();
        }
    }

    fn draw_selection_on_line(&self, ui: &mut egui::Ui, line_idx: usize, galley: &std::sync::Arc<egui::Galley>, line_rect: &egui::Rect) {
        if let Some(selection) = self.editor_state.selection {
            let normalized_selection = selection.normalized();

            let selection_starts_on_this_line = normalized_selection.start.line == line_idx;
            let selection_ends_on_this_line = normalized_selection.end.line == line_idx;
            let selection_spans_this_line = normalized_selection.start.line < line_idx && normalized_selection.end.line > line_idx;

            if selection_starts_on_this_line || selection_ends_on_this_line || selection_spans_this_line {
                let start_char_in_line = if selection_starts_on_this_line {
                    normalized_selection.start.char_idx
                } else {
                    0
                };

                let end_char_in_line = if selection_ends_on_this_line {
                    normalized_selection.end.char_idx
                } else {
                    self.content.line(line_idx).len_chars()
                };

                let galley_ref_selection: &egui::Galley = galley.deref();

                let x_start = galley_ref_selection.rows.get(0)
                    .and_then(|row| row.glyphs.get(start_char_in_line))
                    .map(|glyph_info| glyph_info.pos.x)
                    .unwrap_or(0.0);

                let x_end = galley_ref_selection.rows.get(0)
                    .and_then(|row| {
                        if end_char_in_line == row.glyphs.len() {
                            Some(row.rect.width())
                        } else {
                            row.glyphs.get(end_char_in_line)
                                .map(|glyph_info| glyph_info.pos.x)
                        }
                    })
                    .unwrap_or(0.0);

                let selection_rect = egui::Rect::from_min_max(
                    egui::pos2(x_start, line_rect.top()),
                    egui::pos2(x_end, line_rect.bottom()),
                );
                
                let mut adjusted_selection_rect = selection_rect;
                adjusted_selection_rect = adjusted_selection_rect.translate(line_rect.left_top().to_vec2());

                let selection_color = ui.style().visuals.selection.bg_fill;
                ui.painter().rect_filled(adjusted_selection_rect, 0.0, selection_color);
            }
        }
    }

    fn invalidate_cache_from_line(&mut self, line_idx: usize) {
        for i in line_idx..self.galley_cache.len() {
            self.galley_cache[i] = None;
        }
    }
}