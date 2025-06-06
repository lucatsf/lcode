// src/core/editor.rs

use egui::Vec2;
use ropey::Rope;
// Corrected imports for undo crate v0.52.0
use undo::{Record, Edit};
// Removed: use std::result::Result; // This is no longer needed as Edit trait returns Self::Output

/// Representa a posição do cursor no texto (linha, coluna de caractere).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    pub line: usize,
    pub char_idx: usize, // Índice do caractere dentro da linha
}

/// Representa uma seleção de texto (início e fim do cursor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Selection {
    pub start: Cursor,
    pub end: Cursor,
}

impl Selection {
    pub fn is_active(&self) -> bool {
        self.start != self.end
    }

    /// Retorna a seleção normalizada (start <= end).
    pub fn normalized(&self) -> Self {
        if self.start.line > self.end.line ||
           (self.start.line == self.end.line && self.start.char_idx > self.end.char_idx) {
            Selection { start: self.end, end: self.start }
        } else {
            *self
        }
    }
}


/// Comando de edição para o sistema de desfazer/refazer.
#[derive(Debug)]
enum EditorCommand {
    Insert {
        at_char_idx: usize,
        text: Rope, // Usar Rope para o texto inserido para eficiência
    },
    Delete {
        at_char_idx: usize,
        text: Rope, // Usar Rope para o texto removido
    },
    // Futuras operações (Substituir, etc.)
}

// Corrected UndoCmd (now Edit) implementation for undo v0.52.0
impl Edit for EditorCommand {
    type Target = Rope; // Define the target type for this command
    type Output = (); // The output of the edit operation (often () for side effects)

    fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            EditorCommand::Insert { at_char_idx, text } => {
                target.insert(*at_char_idx, &text.to_string());
            },
            EditorCommand::Delete { at_char_idx, text } => {
                let end_idx = *at_char_idx + text.len_chars();
                target.remove(*at_char_idx..end_idx);
            },
        }
    }

    fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            EditorCommand::Insert { at_char_idx, text } => {
                let end_idx = *at_char_idx + text.len_chars();
                target.remove(*at_char_idx..end_idx);
            },
            EditorCommand::Delete { at_char_idx, text } => {
                target.insert(*at_char_idx, &text.to_string());
            },
        }
    }
}

/// Gerencia o estado de um editor de texto individual.
#[derive(Debug, Default)]
pub struct TextEditor {
    pub cursor: Cursor,
    pub selection: Option<Selection>, // None se não houver seleção
    pub scroll_offset: Vec2, // Para controlar a posição de rolagem
    
    // Histórico de desfazer/refazer
    undo_record: Record<EditorCommand>,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            cursor: Cursor::default(),
            selection: None,
            scroll_offset: Vec2::ZERO,
            undo_record: Record::new(),
        }
    }

    // Métodos de manipulação de texto (operam no Rope da EditorTab pai)
    pub fn insert_char(&mut self, content: &mut Rope, ch: char) {
        // Clear selection first if active, and delete selected text
        if self.selection.is_some() {
            self.delete_selected_text(content);
        }

        let current_char_idx_in_rope = content.line_to_char(self.cursor.line) + self.cursor.char_idx;
        self.undo_record.edit(content, EditorCommand::Insert {
            at_char_idx: current_char_idx_in_rope,
            text: Rope::from(ch.to_string()),
        });
        self.move_cursor_right(content);
        self.selection = None; // Limpa seleção após inserção
    }

    pub fn insert_text(&mut self, content: &mut Rope, text: &str) {
        // Clear selection first if active, and delete selected text
        if self.selection.is_some() {
            self.delete_selected_text(content);
        }

        let current_char_idx_in_rope = content.line_to_char(self.cursor.line) + self.cursor.char_idx;
        self.undo_record.edit(content, EditorCommand::Insert {
            at_char_idx: current_char_idx_in_rope,
            text: Rope::from(text),
        });
        // Move o cursor para o final do texto inserido
        let mut new_line = self.cursor.line;
        let mut new_char_idx = self.cursor.char_idx;

        for (_, c) in text.chars().enumerate() { // Fixed: `i` replaced with `_`
            if c == '\n' {
                new_line += 1;
                new_char_idx = 0;
            } else {
                new_char_idx += 1;
            }
        }
        self.cursor.line = new_line;
        self.cursor.char_idx = new_char_idx;

        // Ensure cursor does not go beyond end of new line if it was at end of previous
        let current_line_len = content.line(self.cursor.line).len_chars();
        self.cursor.char_idx = self.cursor.char_idx.min(current_line_len);

        self.selection = None;
    }

    pub fn delete_char_before_cursor(&mut self, content: &mut Rope) {
        if self.selection.is_some() {
            self.delete_selected_text(content);
            return;
        }

        let current_char_idx_in_rope = content.line_to_char(self.cursor.line) + self.cursor.char_idx;
        if current_char_idx_in_rope > 0 {
            let start_char_idx_to_remove = current_char_idx_in_rope - 1;
            let removed_char_slice = content.slice(start_char_idx_to_remove..current_char_idx_in_rope);
            
            self.undo_record.edit(content, EditorCommand::Delete {
                at_char_idx: start_char_idx_to_remove,
                text: removed_char_slice.into(), // Convert RopeSlice to Rope
            });
            self.move_cursor_left(content);
        }
    }

    pub fn delete_char_after_cursor(&mut self, content: &mut Rope) {
        if self.selection.is_some() {
            self.delete_selected_text(content);
            return;
        }

        let current_char_idx_in_rope = content.line_to_char(self.cursor.line) + self.cursor.char_idx;
        if current_char_idx_in_rope < content.len_chars() {
            let removed_char_slice = content.slice(current_char_idx_in_rope..current_char_idx_in_rope + 1);
            self.undo_record.edit(content, EditorCommand::Delete {
                at_char_idx: current_char_idx_in_rope,
                text: removed_char_slice.into(), // Convert RopeSlice to Rope
            });
            // Cursor não se move após delete "para frente"
        }
    }

    pub fn delete_selected_text(&mut self, content: &mut Rope) {
        if let Some(selection) = self.selection.take() { // take() move a seleção e a torna None
            let normalized_selection = selection.normalized();
            let start_char_idx = content.line_to_char(normalized_selection.start.line) + normalized_selection.start.char_idx;
            let end_char_idx = content.line_to_char(normalized_selection.end.line) + normalized_selection.end.char_idx;
            
            if start_char_idx < end_char_idx {
                let removed_text = content.slice(start_char_idx..end_char_idx).clone();
                self.undo_record.edit(content, EditorCommand::Delete {
                    at_char_idx: start_char_idx,
                    text: removed_text.into(), // Convert RopeSlice to Rope
                });
                self.cursor = normalized_selection.start; // Move cursor para o início da seleção
            }
        }
    }

    // Métodos de movimento do cursor
    pub fn move_cursor_left(&mut self, content: &Rope) {
        self.selection = None;
        if self.cursor.char_idx > 0 {
            self.cursor.char_idx -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.char_idx = content.line(self.cursor.line).len_chars();
        }
    }

    pub fn move_cursor_right(&mut self, content: &Rope) {
        self.selection = None;
        let current_line_len = content.line(self.cursor.line).len_chars();
        if self.cursor.char_idx < current_line_len {
            self.cursor.char_idx += 1;
        } else if self.cursor.line < content.len_lines() - 1 {
            self.cursor.line += 1;
            self.cursor.char_idx = 0;
        }
    }

    pub fn move_cursor_up(&mut self, content: &Rope) {
        self.selection = None;
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            let target_line_len = content.line(self.cursor.line).len_chars();
            self.cursor.char_idx = self.cursor.char_idx.min(target_line_len);
        }
    }

    pub fn move_cursor_down(&mut self, content: &Rope) {
        self.selection = None;
        if self.cursor.line < content.len_lines() - 1 {
            self.cursor.line += 1;
            let target_line_len = content.line(self.cursor.line).len_chars();
            self.cursor.char_idx = self.cursor.char_idx.min(target_line_len);
        }
    }

    pub fn new_line(&mut self, content: &mut Rope) {
        self.delete_selected_text(content); // Remove seleção antes de nova linha
        let current_char_idx_in_rope = content.line_to_char(self.cursor.line) + self.cursor.char_idx;
        self.undo_record.edit(content, EditorCommand::Insert {
            at_char_idx: current_char_idx_in_rope,
            text: Rope::from("\n"),
        });
        self.cursor.line += 1;
        self.cursor.char_idx = 0;
        self.selection = None;
    }

    // Métodos de desfazer/refazer
    pub fn undo(&mut self, content: &mut Rope) -> bool {
        self.selection = None;
        self.undo_record.undo(content).is_some() // Fixed: use .is_some()
    }

    pub fn redo(&mut self, content: &mut Rope) -> bool {
        self.selection = None;
        self.undo_record.redo(content).is_some() // Fixed: use .is_some()
    }

    // Métodos de seleção (ainda bem básicos, serão aprimorados)
    pub fn set_selection_start(&mut self) {
        self.selection = Some(Selection {
            start: self.cursor,
            end: self.cursor,
        });
    }

    pub fn extend_selection(&mut self) {
        if let Some(selection) = &mut self.selection {
            selection.end = self.cursor;
        } else {
            // If no selection is active, start one
            self.set_selection_start();
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }
}