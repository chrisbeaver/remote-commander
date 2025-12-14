use anyhow::Result;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use ssh2::Channel;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

const BUFFER_SIZE: usize = 8192;

pub enum ShellType {
    Local(LocalShell),
    Remote(RemoteShell),
}

pub struct LocalShell {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output_buffer: Arc<Mutex<Vec<u8>>>,
    cached_output: Arc<Mutex<String>>,
}

pub struct RemoteShell {
    channel: Channel,
    output_buffer: Arc<Mutex<Vec<u8>>>,
    _reader_thread: Option<std::thread::JoinHandle<()>>,
}

impl LocalShell {
    pub fn new() -> Result<Self> {
        let pty_system = native_pty_system();
        
        // Get default shell from environment or use /bin/sh
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let cmd = CommandBuilder::new(&shell);
        let _child = pair.slave.spawn_command(cmd)?;
        
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        let cached_output = Arc::new(Mutex::new(String::new()));
        let buffer_clone = Arc::clone(&output_buffer);
        let cache_clone = Arc::clone(&cached_output);
        
        // Spawn thread to read from PTY
        std::thread::spawn(move || {
            let mut buf = [0u8; BUFFER_SIZE];
            loop {
                match reader.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        if let Ok(mut buffer) = buffer_clone.lock() {
                            buffer.extend_from_slice(&buf[..n]);
                            // Keep buffer from growing too large
                            if buffer.len() > 100_000 {
                                buffer.drain(..50_000);
                            }
                            // Update cached string
                            if let Ok(mut cache) = cache_clone.lock() {
                                *cache = String::from_utf8_lossy(&buffer).to_string();
                            }
                        }
                    }
                    Ok(_) => break, // EOF
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            output_buffer,
            cached_output,
        })
    }

    pub fn write_input(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn clear_output(&mut self) {
        if let Ok(mut buffer) = self.output_buffer.lock() {
            buffer.clear();
        }
        if let Ok(mut cache) = self.cached_output.lock() {
            cache.clear();
        }
    }

    pub fn get_output(&self) -> String {
        if let Ok(cache) = self.cached_output.lock() {
            cache.clone()
        } else {
            String::new()
        }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}

impl RemoteShell {
    pub fn new(session: &ssh2::Session) -> Result<Self> {
        let mut channel = session.channel_session()?;
        channel.request_pty("xterm", None, None)?;
        channel.shell()?;
        
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        
        // TODO: Implement background thread for reading
        // For now, we'll skip reading to avoid blocking issues
        
        Ok(Self {
            channel,
            output_buffer,
            _reader_thread: None,
        })
    }

    pub fn write_input(&mut self, data: &[u8]) -> Result<()> {
        self.channel.write_all(data)?;
        self.channel.flush()?;
        Ok(())
    }

    pub fn read_available(&mut self) -> Result<()> {
        // TODO: Implement proper non-blocking read
        // For now, skip reading to avoid blocking the UI thread
        // Remote shell output will be implemented with a background thread
        Ok(())
    }

    pub fn clear_output(&mut self) {
        if let Ok(mut buffer) = self.output_buffer.lock() {
            buffer.clear();
        }
    }

    pub fn get_output(&self) -> String {
        if let Ok(buffer) = self.output_buffer.lock() {
            String::from_utf8_lossy(&buffer).to_string()
        } else {
            String::new()
        }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.channel.request_pty_size(cols as u32, rows as u32, None, None)?;
        Ok(())
    }
}

impl ShellType {
    pub fn write_input(&mut self, data: &[u8]) -> Result<()> {
        match self {
            ShellType::Local(shell) => shell.write_input(data),
            ShellType::Remote(shell) => shell.write_input(data),
        }
    }

    pub fn get_output(&self) -> String {
        match self {
            ShellType::Local(shell) => shell.get_output(),
            ShellType::Remote(shell) => shell.get_output(),
        }
    }

    pub fn clear_output(&mut self) {
        match self {
            ShellType::Local(shell) => shell.clear_output(),
            ShellType::Remote(shell) => shell.clear_output(),
        }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        match self {
            ShellType::Local(shell) => shell.resize(rows, cols),
            ShellType::Remote(shell) => shell.resize(rows, cols),
        }
    }
}
