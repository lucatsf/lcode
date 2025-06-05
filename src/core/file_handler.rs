// src/core/file_handler.rs

use ropey::Rope;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use memmap2::Mmap;

/// Carrega o conteúdo de um arquivo para um Rope, otimizando para arquivos grandes.
///
/// Se o arquivo for menor que 1MB, lê todo o conteúdo para a memória.
/// Para arquivos maiores em sistemas Unix, usa `mmap` para carregamento eficiente.
///
/// # Argumentos
///
/// * `path` - O caminho para o arquivo a ser carregado.
///
/// # Retorno
///
/// Retorna um `Result` contendo o `Rope` com o conteúdo do arquivo em caso de sucesso,
/// ou um `io::Error` em caso de falha.
pub fn load_file_into_rope(path: &Path) -> io::Result<Rope> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_len = metadata.len();

    if file_len < 1024 * 1024 { // 1MB
        let mut buffer = String::new();
        file.take(file_len).read_to_string(&mut buffer)?;
        Ok(Rope::from(buffer))
    } else {
        #[cfg(target_family = "unix")]
        {
            // Em sistemas Unix, podemos usar mmap para arquivos grandes de forma eficiente
            let mmap = unsafe { Mmap::map(&file)? };
            let content_str = std::str::from_utf8(&mmap)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Arquivo não é UTF-8 válido: {}", e)))?;
            Ok(Rope::from(content_str))
        }
        #[cfg(not(target_family = "unix"))]
        {
            // Em outros sistemas, ou como fallback, lemos para a memória
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;
            Ok(Rope::from(buffer))
        }
    }
}