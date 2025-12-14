use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, ConfirmationAction};

pub fn draw_help_popup(frame: &mut Frame, area: Rect) {
    let popup_width = 50;
    let popup_height = 15;
    
    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    let help_text = vec![
        Line::from("Remote Commander - Help"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  ↑/↓       Move selection"),
        Line::from("  PgUp/PgDn Page up/down"),
        Line::from("  Home/End  First/Last item"),
        Line::from("  Enter     Enter directory"),
        Line::from("  Backspace Parent directory"),
        Line::from("  Tab       Switch panels"),
        Line::from(""),
        Line::from("Commands:"),
        Line::from("  F1/h Help    F5/c Copy     F8/d Delete"),
        Line::from("  F3/v View    F6/m Move     F10/q Quit"),
        Line::from("  F4/e Edit    F7/n MkDir"),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help (F1 to close) ")
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().bg(Color::Black));

    // Clear the area first
    frame.render_widget(Clear, popup_area);
    frame.render_widget(help_paragraph, popup_area);
}

pub fn draw_confirmation_popup(frame: &mut Frame, area: Rect, app: &App) {
    let (title, message) = match &app.confirmation_dialog {
        Some(ConfirmationAction::Copy { source, dest_path }) => {
            let msg = format!(
                "Copy '{}' to {}?",
                source.name,
                dest_path.display()
            );
            ("Confirm Copy", msg)
        }
        Some(ConfirmationAction::Move { source, dest_path }) => {
            let msg = format!(
                "Move '{}' to {}?",
                source.name,
                dest_path.display()
            );
            ("Confirm Move", msg)
        }
        Some(ConfirmationAction::Delete { entry }) => {
            let item_type = if entry.is_dir { "directory" } else { "file" };
            let msg = format!(
                "Delete {} '{}'?",
                item_type,
                entry.name
            );
            ("Confirm Delete", msg)
        }
        None => return,
    };

    let popup_width = 60;
    let popup_height = 8;
    
    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    let confirmation_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            message,
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("es   "),
            Span::styled("[N]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("o   "),
            Span::styled("[ESC]", Style::default().fg(Color::Gray)),
            Span::raw(" Cancel"),
        ]),
    ];

    let confirmation_paragraph = Paragraph::new(confirmation_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().bg(Color::Black));

    // Clear the area first
    frame.render_widget(Clear, popup_area);
    frame.render_widget(confirmation_paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_draw_help_popup() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        
        terminal.draw(|frame| {
            let area = frame.area();
            draw_help_popup(frame, area);
        }).unwrap();
        
        // Just verify it doesn't panic
    }
}
