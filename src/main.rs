mod app;
mod file_panel;
mod filesystem;
mod shell;
mod ssh;
mod transfer;
mod ui;

use anyhow::{Context, Result};
use app::App;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ssh::{SshConnection, SshConnectionInfo};
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[command(author, version, about = "Norton Commander-style dual-pane file manager with SSH support")]
struct Args {
    /// Remote connection string (e.g., user@hostname or user@hostname:port)
    #[arg(value_name = "USER@HOST")]
    remote: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // If remote connection specified, establish SSH before entering TUI
    let ssh_connection = if let Some(ref remote_str) = args.remote {
        Some(establish_ssh_connection(remote_str)?)
    } else {
        None
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(args.remote, ssh_connection)?;

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

fn establish_ssh_connection(connection_string: &str) -> Result<SshConnection> {
    let info = SshConnectionInfo::parse(connection_string)?;
    
    println!("Connecting to {}@{}:{}...", info.username, info.hostname, info.port);
    io::stdout().flush()?;

    // First try with SSH key
    match SshConnection::connect(info.clone(), None) {
        Ok(conn) => {
            println!("Connected using SSH key.");
            return Ok(conn);
        }
        Err(_) => {
            // SSH key failed, prompt for password
            println!("SSH key authentication failed or not available.");
        }
    }

    // Prompt for password
    let password = rpassword::prompt_password(format!("{}@{}'s password: ", info.username, info.hostname))
        .context("Failed to read password")?;

    let connection = SshConnection::connect(info, Some(&password))
        .context("SSH connection failed")?;
    
    println!("Connected.");
    
    Ok(connection)
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Use polling with timeout to reduce CPU usage and improve responsiveness
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Handle confirmation dialog keys if active
                    if app.confirmation_dialog.is_some() {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            app.confirm_action()?;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            app.cancel_confirmation();
                        }
                        _ => {}
                    }
                } else if app.show_terminal && app.terminal_input_mode {
                    // Terminal input mode - send ALL keys to shell except Tab/Esc
                    match key.code {
                        KeyCode::Tab | KeyCode::Esc => {
                            // Exit terminal input mode, return to navigation
                            app.exit_terminal_input_mode();
                        }
                        KeyCode::Char(c) => {
                            let _ = app.send_to_shell(c.to_string().as_bytes());
                        }
                        KeyCode::Enter => {
                            let _ = app.send_to_shell(b"\n");
                        }
                        KeyCode::Backspace => {
                            let _ = app.send_to_shell(b"\x7f");
                        }
                        KeyCode::Up => {
                            let _ = app.send_to_shell(b"\x1b[A"); // Up arrow
                        }
                        KeyCode::Down => {
                            let _ = app.send_to_shell(b"\x1b[B"); // Down arrow
                        }
                        KeyCode::Left => {
                            let _ = app.send_to_shell(b"\x1b[D"); // Left arrow
                        }
                        KeyCode::Right => {
                            let _ = app.send_to_shell(b"\x1b[C"); // Right arrow
                        }
                        _ => {}
                    }
                } else {
                    // Navigation mode - normal key handling
                    match key.code {
                        KeyCode::Char('q') | KeyCode::F(10) => return Ok(()),
                        KeyCode::Tab => {
                            app.toggle_active_panel();
                            app.status_message = Some(format!(
                                "Active: {} panel",
                                if app.active_panel == app::ActivePanel::Left { "Left" } else { "Right" }
                            ));
                        }
                        KeyCode::Enter if app.show_terminal => {
                            app.enter_terminal_input_mode();
                        }
                        KeyCode::Up => app.move_selection_up(),
                        KeyCode::Down => app.move_selection_down(),
                        KeyCode::Enter => app.enter_directory()?,
                        KeyCode::Backspace => app.go_parent_directory()?,
                        KeyCode::Home => app.move_to_first(),
                        KeyCode::End => app.move_to_last(),
                        KeyCode::PageUp => app.page_up(),
                        KeyCode::PageDown => app.page_down(),
                        KeyCode::F(1) | KeyCode::Char('h') => app.show_help(),
                        KeyCode::F(4) | KeyCode::Char('e') => app.edit_file()?,
                        KeyCode::F(5) | KeyCode::Char('c') => app.copy_file()?,
                        KeyCode::F(6) | KeyCode::Char('m') => app.move_file()?,
                        KeyCode::F(7) | KeyCode::Char('n') => app.make_directory()?,
                        KeyCode::F(8) | KeyCode::Char('d') => app.delete_file()?,
                        KeyCode::F(9) | KeyCode::Char('t') => app.toggle_terminal(),
                        _ => {}
                    }
                }
                }
            }
        }
    }
}
