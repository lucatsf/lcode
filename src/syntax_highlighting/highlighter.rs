use syntect::parsing::{SyntaxSet, SyntaxReference};
use syntect::highlighting::{ThemeSet, Theme, Style, Color};
use syntect::easy::HighlightLines;


use std::path::Path;

/// Struct para gerenciar o realce de sintaxe.
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    current_theme: Theme,
}

impl SyntaxHighlighter {
    /// Cria uma nova instância do SyntaxHighlighter.
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let current_theme = theme_set.themes["base16-ocean.dark"].clone();

        Self {
            syntax_set,
            theme_set,
            current_theme,
        }
    }

    /// Retorna a referência de sintaxe baseada na extensão do arquivo.
    fn get_syntax_for_file(&self, file_path: &Path) -> &SyntaxReference {
        if let Some(extension) = file_path.extension().and_then(|s| s.to_str()) {
            self.syntax_set.find_syntax_by_extension(extension).unwrap_or_else(|| {
                self.syntax_set.find_syntax_plain_text()
            })
        } else {
            self.syntax_set.find_syntax_plain_text()
        }
    }

    /// Realça uma linha de texto.
    pub fn highlight_line<'a>(&self, line: &'a str, file_path: &Path) -> Vec<(Style, &'a str)> {
        let syntax = self.get_syntax_for_file(file_path);
        let mut highlighter = HighlightLines::new(syntax, &self.current_theme);

        highlighter.highlight_line(line, &self.syntax_set).unwrap_or_default()
    }

    /// Altera o tema atual do realce de sintaxe.
    pub fn set_theme(&mut self, theme_name: &str) {
        if let Some(theme) = self.theme_set.themes.get(theme_name) {
            self.current_theme = theme.clone();
            eprintln!("Tema de realce de sintaxe alterado para: {}", theme_name);
        } else {
            eprintln!("Tema '{}' não encontrado. Mantendo o tema atual.", theme_name);
        }
    }

    /// Lista os temas disponíveis.
    pub fn available_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys().cloned().collect()
    }

    /// Converte uma cor do Syntect para uma cor do Egui.
    pub fn syntect_color_to_egui_color(color: Color) -> egui::Color32 {
        egui::Color32::from_rgb(color.r, color.g, color.b)
    }
}

// Implementação de Default para SyntaxHighlighter
impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}
