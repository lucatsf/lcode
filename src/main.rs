// src/main.rs

use lcode::MyApp;
use egui::FontFamily::Proportional;
use egui_phosphor::{add_to_fonts, Variant};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(1200.0, 800.0))
            .with_title("lcode"),
        ..Default::default()
    };
    eframe::run_native(
        "lcode",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            // Adiciona a fonte 'phosphor' ao egui.
            // Isso permite que você use os ícones do phosphor referenciando seus caracteres Unicode.
            add_to_fonts(&mut fonts, Variant::Regular);

            // REMOVA OU COMENTE ESTAS LINHAS:
            // Estas linhas estão fazendo com que a fonte de ícones seja usada para texto regular,
            // resultando nos símbolos estranhos.
            // fonts
            //     .families
            //     .entry(egui::FontFamily::Proportional)
            //     .or_default()
            //     .insert(0, "phosphor".to_owned());
            // fonts
            //     .families
            //     .entry(egui::FontFamily::Monospace)
            //     .or_default()
            //     .insert(0, "phosphor".to_owned());

            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::<MyApp>::default())
        }),
    )
}