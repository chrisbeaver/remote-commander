mod app;
mod file_panel;
mod filesystem;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Remote connection string (e.g., user@hostname)
    #[arg(value_name = "USER@HOST")]
    remote: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(args.remote)?;

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

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::F(10) => return Ok(()),
                    KeyCode::Tab => app.toggle_active_panel(),
                    KeyCode::Up => app.move_selection_up(),
                    KeyCode::Down => app.move_selection_down(),
                    KeyCode::Enter => app.enter_directory()?,
                    KeyCode::Backspace => app.go_parent_directory()?,
                    KeyCode::Home => app.move_to_first(),
                    KeyCode::End => app.move_to_last(),
                    KeyCode::PageUp => app.page_up(),
                    KeyCode::PageDown => app.page_down(),
                    KeyCode::F(1) => app.show_help(),
                    KeyCode::F(3) => app.view_file()?,
                    KeyCode::F(4) => app.edit_file()?,
                    KeyCode::F(5) => app.copy_file()?,
                    KeyCode::F(6) => app.move_file()?,
                    KeyCode::F(7) => app.make_directory()?,
                    KeyCode::F(8) => app.delete_file()?,
                    _ => {}
                }
            }
        }
    }
}
