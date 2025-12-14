use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{ActivePanel, App};

/// Draw the terminal window at the bottom of the screen
pub fn draw_terminal(frame: &mut Frame, area: Rect, app: &App) {
    let title = match app.active_panel {
        ActivePanel::Left => " Terminal - Local ",
        ActivePanel::Right => {
            if app.remote_connection.is_some() {
                " Terminal - Remote "
            } else {
                " Terminal - Local "
            }
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Green));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Placeholder content - will be replaced with actual terminal emulator
    let placeholder_text = vec![
        Line::from("Terminal emulator - Coming soon"),
        Line::from(""),
        Line::from("This will provide:"),
        Line::from("  • Local shell for left panel"),
        Line::from("  • Remote shell for right panel (when connected)"),
        Line::from("  • Shell switches with TAB key"),
        Line::from(""),
        Line::from("Press F9 to toggle terminal display"),
    ];

    let paragraph = Paragraph::new(placeholder_text)
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(paragraph, inner_area);
}
