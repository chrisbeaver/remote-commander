use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{ActivePanel, App};

/// Main draw function for the application
pub fn draw(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Main layout: panels + status bar + function key bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // Panels area
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Function key bar
        ])
        .split(size);

    // Split panels horizontally
    let panel_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[0]);

    // Calculate visible rows for panels (accounting for borders)
    let visible_rows = panel_chunks[0].height.saturating_sub(2) as usize;
    app.set_visible_rows(visible_rows);

    // Draw left panel
    draw_panel(
        frame,
        panel_chunks[0],
        &app.left_panel,
        "Local",
        app.active_panel == ActivePanel::Left,
    );

    // Draw right panel
    let right_title = if let Some(ref remote) = app.remote_connection {
        format!("Remote: {}", remote)
    } else {
        "Local".to_string()
    };
    draw_panel(
        frame,
        panel_chunks[1],
        &app.right_panel,
        &right_title,
        app.active_panel == ActivePanel::Right,
    );

    // Draw status bar
    draw_status_bar(frame, main_chunks[1], app);

    // Draw function key bar
    draw_function_bar(frame, main_chunks[2]);

    // Draw help popup if active
    if app.show_help {
        draw_help_popup(frame, size);
    }
}

fn draw_panel(
    frame: &mut Frame,
    area: Rect,
    panel: &crate::file_panel::FilePanel,
    title: &str,
    is_active: bool,
) {
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let title_with_path = format!(" {} - {} ", title, panel.current_path.display());

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_with_path)
        .border_style(border_style);

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Create list items from entries
    let items: Vec<ListItem> = panel
        .visible_entries()
        .map(|(idx, entry)| {
            let is_selected = idx == panel.selected_index;
            
            // Format the line: name | size | date
            let name = if entry.is_dir {
                format!("[{}]", entry.name)
            } else {
                entry.name.clone()
            };

            // Truncate name if too long
            let max_name_len = inner_area.width.saturating_sub(25) as usize;
            let display_name = if name.len() > max_name_len {
                format!("{}...", &name[..max_name_len.saturating_sub(3)])
            } else {
                name
            };

            let size_str = entry.format_size();
            let date_str = entry.format_date();

            let line_content = format!(
                "{:<width$} {:>7} {}",
                display_name,
                size_str,
                date_str,
                width = max_name_len
            );

            let style = if is_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(Span::styled(line_content, style)))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner_area);
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let message = app
        .status_message
        .as_deref()
        .unwrap_or("");

    let paragraph = Paragraph::new(Line::from(Span::styled(
        format!(" {}", message),
        Style::default().fg(Color::Yellow).bg(Color::DarkGray),
    )))
    .style(Style::default().bg(Color::DarkGray));

    frame.render_widget(paragraph, area);
}

fn draw_function_bar(frame: &mut Frame, area: Rect) {
    let function_keys = vec![
        ("F1/h", "Help"),
        ("F2", "Menu"),
        ("F3/v", "View"),
        ("F4/e", "Edit"),
        ("F5/c", "Copy"),
        ("F6/m", "Move"),
        ("F7/n", "New"),
        ("F8/d", "Del"),
        ("F9", "Term"),
        ("F10/q", "Quit"),
    ];

    let spans: Vec<Span> = function_keys
        .iter()
        .flat_map(|(key, label)| {
            vec![
                Span::styled(
                    format!("{}", key),
                    Style::default().bg(Color::Cyan).fg(Color::Black),
                ),
                Span::styled(
                    format!("{:<6}", label),
                    Style::default().bg(Color::Black).fg(Color::White),
                ),
            ]
        })
        .collect();

    let paragraph = Paragraph::new(Line::from(spans));
    frame.render_widget(paragraph, area);
}

fn draw_help_popup(frame: &mut Frame, area: Rect) {
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
    frame.render_widget(ratatui::widgets::Clear, popup_area);
    frame.render_widget(help_paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_draw_function_bar() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        
        terminal.draw(|frame| {
            let area = frame.area();
            draw_function_bar(frame, area);
        }).unwrap();
        
        // Just verify it doesn't panic
    }

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
