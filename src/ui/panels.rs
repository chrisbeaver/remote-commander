use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::file_panel::FilePanel;

pub fn draw_panel(
    frame: &mut Frame,
    area: Rect,
    panel: &FilePanel,
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
