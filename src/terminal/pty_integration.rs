// src/terminal/pty_integration.rs

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Command, Child}; // Usamos Command e Child do tokio::process
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

// Novas importações para a crate `portable-pty`
use portable_pty::{PtySize, CommandBuilder, PtySystem, native_pty_system, MasterPty, PtyPair, Child as PortablePtyChild};

// Custom async writer wrapper for MasterPty
struct PtyAsyncWriter {
    writer: Box<dyn std::io::Write + Send>,
}

impl PtyAsyncWriter {
    fn new(master: Box<dyn MasterPty + Send>) -> io::Result<Self> {
        // Get a writer from the master PTY
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
        // Use blocking write for now - this is not ideal but works
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

// Mensagens que o terminal enviará para a UI
#[derive(Debug)]
pub enum TerminalOutput {
    /// Bytes brutos recebidos do PTY
    Data(Vec<u8>),
    /// O terminal foi encerrado
    Exited(Option<i32>),
    /// Erro no PTY ou no shell
    Error(io::Error),
}

/// Trait para abstrair a interação com o PTY.
/// Isso pode ser útil para futuros mocks ou diferentes implementações de PTY.
#[async_trait]
pub trait PseudoTerminal {
    /// Inicia um novo processo de shell no PTY.
    async fn spawn_shell(&mut self, working_directory: Option<PathBuf>) -> io::Result<()>;

    /// Envia dados para o shell através do PTY.
    async fn write_to_pty(&mut self, data: &[u8]) -> io::Result<usize>;

    /// Lê dados do shell através do PTY.
    async fn read_from_pty(&mut self) -> io::Result<Vec<u8>>;

    fn output_receiver(&mut self) -> mpsc::Receiver<TerminalOutput>;
}

/// Implementação de `PseudoTerminal` usando `portable-pty`.
pub struct PortablePtyTerminal {
    pty_system: Box<dyn PtySystem + Send>,
    master_pty: Option<Box<dyn MasterPty + Send>>,
    reader: Option<Pin<Box<dyn tokio::io::AsyncRead + Send>>>,
    writer: Option<Pin<Box<dyn tokio::io::AsyncWrite + Send>>>,
    shell_child: Option<Box<dyn PortablePtyChild + Send + Sync>>,
    output_tx: mpsc::Sender<TerminalOutput>,
    read_task_handle: Option<JoinHandle<()>>,
}

impl PortablePtyTerminal {
    pub fn new() -> (Self, mpsc::Receiver<TerminalOutput>) {
        let (output_tx, output_rx) = mpsc::channel(1000);
        let instance = Self {
            pty_system: native_pty_system(),
            master_pty: None,
            reader: None,
            writer: None,
            shell_child: None,
            output_tx,
            read_task_handle: None,
        };
        (instance, output_rx)
    }

    /// Spawna uma tarefa assíncrona para ler do PTY mestre e enviar para o canal de output.
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

    /// Helper function to create async streams from MasterPty
    fn create_async_streams(master: Box<dyn MasterPty + Send>) -> io::Result<(Pin<Box<dyn tokio::io::AsyncRead + Send>>, Pin<Box<dyn tokio::io::AsyncWrite + Send>>)> {
        // Get the reader from MasterPty
        let reader = master.try_clone_reader()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        // Create async writer before consuming master
        let async_writer = PtyAsyncWriter::new(master)?;

        // Create a simple async reader using a channel-based approach
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(1024);

        // Spawn a task to read from the sync reader and send to the channel
        let _read_handle = tokio::task::spawn_blocking(move || {
            use std::io::Read;
            let mut reader = reader;
            let mut buffer = vec![0u8; 4096];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let data = buffer[..n].to_vec();
                        if tx.blocking_send(data).is_err() {
                            break; // Channel closed
                        }
                    },
                    Err(_) => break,
                }
            }
        });

        // Create an async reader that reads from the channel
        let async_reader = futures::stream::unfold(rx, |mut rx| async move {
            match rx.recv().await {
                Some(data) => {
                    // Convert Vec<u8> to Bytes which implements Buf
                    let bytes = bytes::Bytes::from(data);
                    Some((Ok::<bytes::Bytes, std::io::Error>(bytes), rx))
                },
                None => None,
            }
        });

        // Convert stream to AsyncRead
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

        // Create async streams from the master PTY
        let (reader_stream, writer_stream) = Self::create_async_streams(pty_pair.master)?;

        self.reader = Some(reader_stream);
        self.writer = Some(writer_stream);

        eprintln!("Shell spawned successfully.");
        self.spawn_read_task();

        Ok(())
    }

    async fn write_to_pty(&mut self, data: &[u8]) -> io::Result<usize> {
        if let Some(writer) = &mut self.writer {
            writer.write_all(data).await?;
            writer.flush().await?;
            Ok(data.len())
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "PTY writer not active"))
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
    pub command_tx: mpsc::Sender<String>,
    command_rx_pty: mpsc::Receiver<String>,
}

impl Terminal {
    pub fn new() -> Self {
        let (pty_instance, output_rx_from_pty) = PortablePtyTerminal::new();
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

                let tx_command = self.command_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = tx_command.send(command).await {
                        eprintln!("Erro ao enviar comando para o PTY: {}", e);
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

        // Processar comandos da UI para o PTY
        while let Ok(command) = self.command_rx_pty.try_recv() {
            let command_with_newline = format!("{}\n", command);
            let data = command_with_newline.as_bytes().to_vec();

            // Create a clone of the PTY for async operations
            let mut pty_clone = &mut self.pty;
            let data_clone = data.clone();

            // Use a blocking approach since we're in the UI thread
            if let Some(_) = &mut pty_clone.writer {
                let rt = tokio::runtime::Handle::current();
                if let Err(e) = rt.block_on(pty_clone.write_to_pty(&data_clone)) {
                    eprintln!("Erro ao escrever no PTY: {}", e);
                }
            }
        }
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
                    // Send error to UI
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
        self.pty.master_pty = None;
        self.pty.reader = None;
        self.pty.writer = None;
        self.is_open = false;
        eprintln!("Terminal parado.");
    }
}