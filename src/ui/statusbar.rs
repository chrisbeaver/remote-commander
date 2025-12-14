use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;

pub fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
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

pub fn draw_function_bar(frame: &mut Frame, area: Rect) {
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
}
