use eframe::egui;
use ropey::Rope;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use memmap2::Mmap;

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
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

// Constantes para otimização da renderização
const LINE_HEIGHT: f32 = 16.0; // Altura aproximada de uma linha de texto em pixels
const LINE_NUMBER_GUTTER_WIDTH: f32 = 60.0; // Largura para os números de linha
const SIDE_PANEL_WIDTH: f32 = 200.0; // Largura inicial da barra lateral

struct MyApp {
    editor_content: Rope,
    file_path: Option<String>,
}

impl Default for MyApp {
    fn default() -> Self {
        let initial_text = "Hello, lcode!\n\nEste é o nosso editor de código minimalista.\n\nTentando carregar um arquivo grande...\n".to_string();
        let mut app = Self {
            editor_content: Rope::from(initial_text),
            file_path: None,
        };

        let test_path = Path::new("test_large_file.txt");
        match load_file_into_rope(test_path) {
            Ok(rope) => {
                app.editor_content = rope;
                app.file_path = Some(test_path.to_string_lossy().into_owned());
                eprintln!("Arquivo '{}' carregado com sucesso!", test_path.display());
            },
            Err(e) => {
                eprintln!("Erro ao carregar o arquivo '{}': {}", test_path.display(), e);
                eprintln!("Usando conteúdo inicial padrão.");
            }
        }
        app
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Painel Lateral Esquerdo (para o explorador de arquivos)
        egui::SidePanel::left("file_explorer_panel")
            .min_width(SIDE_PANEL_WIDTH)
            .default_width(SIDE_PANEL_WIDTH)
            .show(ctx, |ui| {
                ui.heading("Explorador de Arquivos");
                ui.separator();
                ui.label("Conteúdo do diretório virá aqui.");
            });

        // 2. Painel Central (para o editor de texto)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Editor: {}", self.file_path.as_deref().unwrap_or("[No File Loaded]")));
            ui.separator();

            egui::ScrollArea::vertical().show_rows(ui, LINE_HEIGHT, self.editor_content.len_lines(), |ui, row_range| {
                // Layout horizontal para números de linha e texto
                ui.horizontal(|ui| {
                    // Gutter para números de linha
                    ui.vertical(|ui| {
                        ui.set_width(LINE_NUMBER_GUTTER_WIDTH);
                        ui.spacing_mut().item_spacing.y = 0.0;

                        for i in row_range.start..row_range.end {
                            ui.monospace(format!("{:>4}", i + 1));
                        }
                    });

                    // Painel de texto do editor
                    // O `ui.allocate_space(ui.available_size())` é uma forma de "consumir" todo o espaço
                    // horizontal e vertical disponível, mas não é a forma ideal para renderizar texto.
                    // Para garantir que o texto se estenda, vamos usar `ui.add_sized` ou simplesmente
                    // garantir que o layout preencha. A melhor forma é usar `ui.with_layout`
                    // com um layout que force a expansão horizontal.

                    // Garante que o painel de texto use a largura restante disponível.
                    // Isso é feito dentro de um sub-vertical layout para cada linha.
                    ui.vertical(|ui| {
                        // `ui.set_width(ui.available_width());` é uma forma mais direta de dizer
                        // para este sub-painel vertical ocupar toda a largura restante.
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 0.0;

                        for line in self.editor_content.lines_at(row_range.start).take(row_range.len()) {
                            // Para garantir que o `label` se expanda, podemos colocá-lo dentro
                            // de um `Frame` ou `Horizontal` com `fill_to_content` ou usando
                            // `allocate_exact_size` com `ui.available_width()`.
                            // Uma maneira simples é garantir que o label em si seja tratado
                            // como um elemento que "pode" expandir, ou o container.
                            // O `ui.set_width` acima já ajuda.

                            // Agora, para o texto em si, podemos usar um truque com `monospace` e
                            // `with_layout` para garantir que o texto tente preencher a largura.
                            // No entanto, para um editor de texto, o `TextEdit` é quem faria isso.
                            // Para a renderização atual, apenas o `monospace` label:
                            ui.monospace(line.as_str().unwrap_or(""));
                            // Se a linha for mais curta que a largura disponível, o label por si só
                            // não vai preencher o espaço. Mas `ui.set_width` no vertical pai
                            // já faz o painel se estender.

                            // Para garantir que o *fundo* da linha do editor se estenda,
                            // podemos usar `ui.painter().rect_filled` para desenhar um retângulo
                            // com a largura total disponível e a altura da linha, como uma
                            // "caixa de texto" de fundo. Mas isso é um nível de detalhe que
                            // pode ser implementado mais tarde na renderização do editor.
                            // Por enquanto, o `ui.set_width` no `ui.vertical` já deve ajudar a
                            // que o painel do editor em si ocupe o espaço.
                        }
                        // Um hack para preencher o espaço restante horizontalmente
                        // ui.add_space(ui.available_width());
                    });
                });
            });
        });
    }
}

// Função para carregar um arquivo no Rope usando mmap (inalterada)
fn load_file_into_rope(path: &Path) -> io::Result<Rope> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_len = metadata.len();

    if file_len < 1024 * 1024 {
        let mut buffer = String::new();
        file.take(file_len).read_to_string(&mut buffer)?;
        Ok(Rope::from(buffer))
    } else {
        #[cfg(target_family = "unix")]
        {
            let mmap = unsafe { Mmap::map(&file)? };
            let content_str = std::str::from_utf8(&mmap)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Arquivo não é UTF-8 válido: {}", e)))?;
            Ok(Rope::from(content_str))
        }
        #[cfg(not(target_family = "unix"))]
        {
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;
            Ok(Rope::from(buffer))
        }
    }
}