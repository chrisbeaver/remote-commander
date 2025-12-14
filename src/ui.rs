mod panels;
mod popups;
mod statusbar;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::app::{ActivePanel, App};

// Re-export submodule functions for external use if needed
pub use panels::draw_panel;
pub use popups::{draw_confirmation_popup, draw_help_popup};
pub use statusbar::{draw_function_bar, draw_status_bar};

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
    panels::draw_panel(
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
    panels::draw_panel(
        frame,
        panel_chunks[1],
        &app.right_panel,
        &right_title,
        app.active_panel == ActivePanel::Right,
    );

    // Draw status bar
    statusbar::draw_status_bar(frame, main_chunks[1], app);

    // Draw function key bar
    statusbar::draw_function_bar(frame, main_chunks[2]);

    // Draw help popup if active
    if app.show_help {
        popups::draw_help_popup(frame, size);
    }

    // Draw confirmation dialog if active
    if app.confirmation_dialog.is_some() {
        popups::draw_confirmation_popup(frame, size, app);
    }
}
