// src/terminal/pty_integration.rs

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use async_trait::async_trait;
use std::io;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use egui::Widget;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::StreamExt;
use bytes::Buf;

use portable_pty::{PtySize, CommandBuilder, PtySystem, native_pty_system, MasterPty, PtyPair, Child as PortablePtyChild};

struct PtyAsyncWriter {
    writer: Box<dyn std::io::Write + Send>,
}

impl PtyAsyncWriter {
    fn new(master: Box<dyn MasterPty + Send>) -> io::Result<Self> {
        let writer = master.take_writer()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(Self { writer })
    }
}

impl tokio::io::AsyncWrite for PtyAsyncWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match self.writer.write(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match self.writer.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Debug)]
pub enum TerminalOutput {
    Data(Vec<u8>),
    Exited(Option<i32>),
    Error(io::Error),
}

#[async_trait]
pub trait PseudoTerminal {
    async fn spawn_shell(&mut self, working_directory: Option<PathBuf>) -> io::Result<()>;
    async fn write_to_pty(&mut self, data: &[u8]) -> io::Result<usize>;
    async fn read_from_pty(&mut self) -> io::Result<Vec<u8>>;
    fn output_receiver(&mut self) -> mpsc::Receiver<TerminalOutput>;
}

pub struct PortablePtyTerminal {
    pty_system: Box<dyn PtySystem + Send>,
    master_pty: Option<Box<dyn MasterPty + Send>>,
    reader: Option<Pin<Box<dyn tokio::io::AsyncRead + Send>>>,
    writer: Option<Pin<Box<dyn tokio::io::AsyncWrite + Send>>>,
    shell_child: Option<Box<dyn PortablePtyChild + Send + Sync>>,
    output_tx: mpsc::Sender<TerminalOutput>,
    read_task_handle: Option<JoinHandle<()>>,
    // NOVO: Canal para receber dados para escrita no PTY
    write_tx: mpsc::Sender<Vec<u8>>,
    write_rx: Option<mpsc::Receiver<Vec<u8>>>,
    write_task_handle: Option<JoinHandle<()>>,
}

impl PortablePtyTerminal {
    pub fn new() -> (Self, mpsc::Receiver<TerminalOutput>, mpsc::Sender<Vec<u8>>) { // NOVO: Retorna também o Sender para escrita
        let (output_tx, output_rx) = mpsc::channel(1000);
        let (write_tx, write_rx) = mpsc::channel(100); // NOVO: Canal para escrita

        let instance = Self {
            pty_system: native_pty_system(),
            master_pty: None,
            reader: None,
            writer: None,
            shell_child: None,
            output_tx,
            read_task_handle: None,
            write_tx: write_tx.clone(), // NOVO: Clone do sender para a própria struct
            write_rx: Some(write_rx), // NOVO: Receiver para a struct
            write_task_handle: None, // NOVO: Handle para a tarefa de escrita
        };
        (instance, output_rx, write_tx) // NOVO: Retorna o write_tx
    }

    fn spawn_read_task(&mut self) {
        if let Some(mut reader) = self.reader.take() {
            let tx = self.output_tx.clone();

            let handle = tokio::spawn(async move {
                let mut buffer = vec![0; 4096];
                loop {
                    tokio::select! {
                        read_result = reader.read(&mut buffer) => {
                            match read_result {
                                Ok(0) => {
                                    eprintln!("PTY master reader EOF.");
                                    if let Err(e) = tx.send(TerminalOutput::Exited(None)).await {
                                        eprintln!("Erro ao enviar TerminalOutput::Exited: {}", e);
                                    }
                                    break;
                                },
                                Ok(n) => {
                                    if let Err(e) = tx.send(TerminalOutput::Data(buffer[..n].to_vec())).await {
                                        eprintln!("Erro ao enviar TerminalOutput::Data: {}", e);
                                    }
                                },
                                Err(e) => {
                                    eprintln!("Erro de leitura do PTY mestre: {}", e);
                                    if let Err(send_err) = tx.send(TerminalOutput::Error(e)).await {
                                        eprintln!("Erro ao enviar TerminalOutput::Error: {}", send_err);
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            });
            self.read_task_handle = Some(handle);
        }
    }

    // NOVO: Tarefa para lidar com a escrita no PTY
    fn spawn_write_task(&mut self) {
        if let Some(mut writer) = self.writer.take() {
            if let Some(mut rx) = self.write_rx.take() {
                let handle = tokio::spawn(async move {
                    while let Some(data) = rx.recv().await {
                        if let Err(e) = writer.write_all(&data).await {
                            eprintln!("Erro ao escrever no PTY: {}", e);
                            break;
                        }
                        if let Err(e) = writer.flush().await {
                            eprintln!("Erro ao dar flush no PTY: {}", e);
                            break;
                        }
                    }
                    eprintln!("Tarefa de escrita do PTY encerrada.");
                });
                self.write_task_handle = Some(handle);
            }
        }
    }

    fn create_async_streams(master: Box<dyn MasterPty + Send>) -> io::Result<(Pin<Box<dyn tokio::io::AsyncRead + Send>>, Pin<Box<dyn tokio::io::AsyncWrite + Send>>)> {
        let reader = master.try_clone_reader()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let async_writer = PtyAsyncWriter::new(master)?;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(1024);

        let _read_handle = tokio::task::spawn_blocking(move || {
            use std::io::Read;
            let mut reader = reader;
            let mut buffer = vec![0u8; 4096];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = buffer[..n].to_vec();
                        if tx.blocking_send(data).is_err() {
                            break;
                        }
                    },
                    Err(_) => break,
                }
            }
        });

        let async_reader = futures::stream::unfold(rx, |mut rx| async move {
            match rx.recv().await {
                Some(data) => {
                    let bytes = bytes::Bytes::from(data);
                    Some((Ok::<bytes::Bytes, std::io::Error>(bytes), rx))
                },
                None => None,
            }
        });

        let async_reader = tokio_util::io::StreamReader::new(async_reader);

        Ok((
            Box::pin(async_reader) as Pin<Box<dyn tokio::io::AsyncRead + Send>>,
            Box::pin(async_writer) as Pin<Box<dyn tokio::io::AsyncWrite + Send>>,
        ))
    }
}

impl Drop for PortablePtyTerminal {
    fn drop(&mut self) {
        if let Some(handle) = self.read_task_handle.take() {
            handle.abort();
            eprintln!("Tarefa de leitura do PTY abortada.");
        }
        // NOVO: Abortar a tarefa de escrita também
        if let Some(handle) = self.write_task_handle.take() {
            handle.abort();
            eprintln!("Tarefa de escrita do PTY abortada.");
        }
        if let Some(mut child) = self.shell_child.take() {
            eprintln!("Terminando processo PTY (shell)...");
            let _ = child.kill();
        }
        self.master_pty = None;
        self.reader = None;
        self.writer = None;
    }
}

#[async_trait]
impl PseudoTerminal for PortablePtyTerminal {
    async fn spawn_shell(&mut self, working_directory: Option<PathBuf>) -> io::Result<()> {
        let pty_pair = self.pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        eprintln!("Spawning shell: {}", shell);

        let mut cmd_builder = CommandBuilder::new(&shell);
        if let Some(dir) = working_directory {
            eprintln!("Setting working directory to: {:?}", dir);
            cmd_builder.cwd(dir);
        }

        let shell_child = pty_pair.slave.spawn_command(cmd_builder)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        self.shell_child = Some(shell_child);

        let (reader_stream, writer_stream) = Self::create_async_streams(pty_pair.master)?;

        self.reader = Some(reader_stream);
        self.writer = Some(writer_stream);

        eprintln!("Shell spawned successfully.");
        self.spawn_read_task();
        self.spawn_write_task(); // NOVO: Iniciar a tarefa de escrita

        Ok(())
    }

    async fn write_to_pty(&mut self, data: &[u8]) -> io::Result<usize> {
        // Agora, a escrita é feita através do canal para a tarefa de escrita
        if let Err(_) = self.write_tx.send(data.to_vec()).await {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "Failed to send data to PTY writer task"))
        } else {
            Ok(data.len())
        }
    }

    async fn read_from_pty(&mut self) -> io::Result<Vec<u8>> {
        Err(io::Error::new(io::ErrorKind::Other, "read_from_pty should not be called directly"))
    }

    fn output_receiver(&mut self) -> mpsc::Receiver<TerminalOutput> {
        panic!("output_receiver() não pode ser chamado, o Receiver é movido na construção de PortablePtyTerminal");
    }
}

// Struct para o estado do terminal na UI
pub struct Terminal {
    pub pty: PortablePtyTerminal,
    pub input_buffer: String,
    pub output_buffer: String,
    pub is_open: bool,
    pub scroll_offset: f32,
    pub terminal_output_rx_ui: mpsc::Receiver<TerminalOutput>,
    pub command_tx: mpsc::Sender<String>, // Este ainda é o canal da UI para a lógica de `Terminal`
    command_rx_pty: mpsc::Receiver<String>,
    pty_write_tx: mpsc::Sender<Vec<u8>>, // NOVO: Sender para a tarefa de escrita do PTY
}

impl Terminal {
    pub fn new() -> Self {
        let (pty_instance, output_rx_from_pty, pty_write_tx) = PortablePtyTerminal::new(); // NOVO: Captura o pty_write_tx
        let (command_tx, command_rx_pty) = mpsc::channel(100);

        Terminal {
            pty: pty_instance,
            input_buffer: String::new(),
            output_buffer: String::new(),
            is_open: false,
            scroll_offset: 0.0,
            terminal_output_rx_ui: output_rx_from_pty,
            command_tx,
            command_rx_pty,
            pty_write_tx, // NOVO: Inicializa o pty_write_tx
        }
    }

    /// Desenha a interface do terminal.
    pub fn ui(&mut self, ui: &mut egui::Ui, current_dir: Option<PathBuf>) {
        ui.heading("Terminal Integrado");
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui_scroll| {
                ui_scroll.add(egui::Label::new(egui::RichText::new(&self.output_buffer).monospace()));
            });

        ui.separator();

        ui.horizontal(|ui_input| {
            let text_edit_response = egui::TextEdit::singleline(&mut self.input_buffer)
                .desired_width(ui_input.available_width() - 50.0)
                .lock_focus(true)
                .hint_text("Digite comandos aqui...")
                .ui(ui_input);

            if text_edit_response.lost_focus() && ui_input.input(|i| i.key_pressed(egui::Key::Enter)) {
                let command = self.input_buffer.clone();
                self.output_buffer.push_str(&format!("> {}\n", command));
                self.input_buffer.clear();

                // NOVO: Envia o comando diretamente para a tarefa de escrita do PTY
                let pty_write_tx_clone = self.pty_write_tx.clone();
                let command_with_newline = format!("{}\n", command);
                let data = command_with_newline.as_bytes().to_vec();

                tokio::spawn(async move {
                    if let Err(e) = pty_write_tx_clone.send(data).await {
                        eprintln!("Erro ao enviar comando para a tarefa de escrita do PTY: {}", e);
                    }
                });
                text_edit_response.request_focus();
            }
        });

        // Processar mensagens do PTY no loop de update da UI
        while let Ok(msg) = self.terminal_output_rx_ui.try_recv() {
            match msg {
                TerminalOutput::Data(data) => {
                    if let Ok(s) = String::from_utf8(data) {
                        self.output_buffer.push_str(&s);
                        ui.ctx().request_repaint();
                    } else {
                        eprintln!("Received non-UTF8 data from PTY");
                    }
                },
                TerminalOutput::Exited(code) => {
                    self.output_buffer.push_str(&format!("\nShell exited with code: {:?}\n", code));
                    ui.ctx().request_repaint();
                },
                TerminalOutput::Error(e) => {
                    self.output_buffer.push_str(&format!("\nTerminal Error: {}\n", e));
                    ui.ctx().request_repaint();
                }
            }
        }

        // REMOVIDO: Este bloco agora é desnecessário, pois a escrita é feita diretamente acima
        // while let Ok(command) = self.command_rx_pty.try_recv() {
        //     let command_with_newline = format!("{}\n", command);
        //     let data = command_with_newline.as_bytes().to_vec();
        //     let mut pty_clone = &mut self.pty;
        //     let data_clone = data.clone();
        //     if let Some(_) = &mut pty_clone.writer {
        //         let rt = tokio::runtime::Handle::current();
        //         if let Err(e) = rt.block_on(pty_clone.write_to_pty(&data_clone)) {
        //             eprintln!("Erro ao escrever no PTY: {}", e);
        //         }
        //     }
        // }
    }

    /// Inicia o terminal com o diretório de trabalho especificado.
    pub fn start(&mut self, current_dir: Option<PathBuf>) {
        if self.pty.shell_child.is_none() {
            let runtime = tokio::runtime::Handle::current();
            let mut pty = &mut self.pty;

            match runtime.block_on(pty.spawn_shell(current_dir)) {
                Ok(()) => {
                    self.is_open = true;
                    eprintln!("Terminal iniciado.");
                },
                Err(e) => {
                    eprintln!("Erro ao iniciar terminal: {}", e);
                    let tx = self.pty.output_tx.clone();
                    tokio::spawn(async move {
                        let _ = tx.send(TerminalOutput::Error(e)).await;
                    });
                }
            }
        } else {
            eprintln!("Terminal já está em execução.");
        }
    }

    /// Para o terminal.
    pub fn stop(&mut self) {
        if let Some(mut child) = self.pty.shell_child.take() {
            eprintln!("Parando processo PTY (shell)...");
            let _ = child.kill();
        }
        if let Some(handle) = self.pty.read_task_handle.take() {
            handle.abort();
        }
        // NOVO: Abortar a tarefa de escrita
        if let Some(handle) = self.pty.write_task_handle.take() {
            handle.abort();
        }
        self.pty.master_pty = None;
        self.pty.reader = None;
        self.pty.writer = None;
        self.is_open = false;
        eprintln!("Terminal parado.");
    }
}