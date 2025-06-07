// src/lib.rs

pub mod core;
pub mod file_explorer;
pub mod syntax_highlighting;
pub mod ui;
pub mod terminal; // <--- ADICIONE ESTA LINHA

pub use ui::app::MyApp; // Exporta MyApp para ser usado em main.rs