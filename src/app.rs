use anyhow::Result;
use std::path::PathBuf;

use crate::file_panel::FilePanel;
use crate::filesystem::LocalFileSystem;
use crate::ssh::{RemoteFileSystem, SshConnection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Left,
    Right,
}

pub struct App {
    pub left_panel: FilePanel,
    pub right_panel: FilePanel,
    pub active_panel: ActivePanel,
    pub remote_connection: Option<String>,
    pub show_help: bool,
    pub status_message: Option<String>,
    pub visible_rows: usize,
}

impl App {
    pub fn new(remote_connection: Option<String>, ssh_connection: Option<SshConnection>) -> Result<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        
        let left_panel = FilePanel::new(LocalFileSystem::new(), home.clone())?;
        
        // If SSH connection provided, use remote filesystem for right panel
        let right_panel = if let Some(ref ssh_conn) = ssh_connection {
            let remote_fs = RemoteFileSystem::new(ssh_conn);
            let remote_home = ssh_conn.home_dir.clone();
            FilePanel::new(remote_fs, remote_home)?
        } else {
            FilePanel::new(LocalFileSystem::new(), home)?
        };

        Ok(Self {
            left_panel,
            right_panel,
            active_panel: ActivePanel::Left,
            remote_connection,
            show_help: false,
            status_message: None,
            visible_rows: 20, // Will be updated by UI
        })
    }

    pub fn active_panel_mut(&mut self) -> &mut FilePanel {
        match self.active_panel {
            ActivePanel::Left => &mut self.left_panel,
            ActivePanel::Right => &mut self.right_panel,
        }
    }

    pub fn active_panel(&self) -> &FilePanel {
        match self.active_panel {
            ActivePanel::Left => &self.left_panel,
            ActivePanel::Right => &self.right_panel,
        }
    }

    pub fn inactive_panel(&self) -> &FilePanel {
        match self.active_panel {
            ActivePanel::Left => &self.right_panel,
            ActivePanel::Right => &self.left_panel,
        }
    }

    pub fn toggle_active_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Left => ActivePanel::Right,
            ActivePanel::Right => ActivePanel::Left,
        };
    }

    pub fn move_selection_up(&mut self) {
        let panel = self.active_panel_mut();
        if panel.selected_index > 0 {
            panel.selected_index -= 1;
            panel.adjust_scroll();
        }
    }

    pub fn move_selection_down(&mut self) {
        let panel = self.active_panel_mut();
        if panel.selected_index < panel.entries.len().saturating_sub(1) {
            panel.selected_index += 1;
            panel.adjust_scroll();
        }
    }

    pub fn move_to_first(&mut self) {
        let panel = self.active_panel_mut();
        panel.selected_index = 0;
        panel.scroll_offset = 0;
    }

    pub fn move_to_last(&mut self) {
        let panel = self.active_panel_mut();
        panel.selected_index = panel.entries.len().saturating_sub(1);
        panel.adjust_scroll();
    }

    pub fn page_up(&mut self) {
        let panel = self.active_panel_mut();
        let page_size = panel.visible_rows.saturating_sub(1);
        panel.selected_index = panel.selected_index.saturating_sub(page_size);
        panel.adjust_scroll();
    }

    pub fn page_down(&mut self) {
        let panel = self.active_panel_mut();
        let page_size = panel.visible_rows.saturating_sub(1);
        panel.selected_index = (panel.selected_index + page_size).min(panel.entries.len().saturating_sub(1));
        panel.adjust_scroll();
    }

    pub fn enter_directory(&mut self) -> Result<()> {
        let panel = self.active_panel_mut();
        if let Some(entry) = panel.entries.get(panel.selected_index).cloned() {
            if entry.is_dir {
                panel.change_directory(&entry.path)?;
            }
        }
        Ok(())
    }

    pub fn go_parent_directory(&mut self) -> Result<()> {
        let panel = self.active_panel_mut();
        if let Some(parent) = panel.current_path.parent() {
            let parent_path = parent.to_path_buf();
            panel.change_directory(&parent_path)?;
        }
        Ok(())
    }

    pub fn show_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn view_file(&mut self) -> Result<()> {
        self.status_message = Some("View: Not yet implemented".to_string());
        Ok(())
    }

    pub fn edit_file(&mut self) -> Result<()> {
        self.status_message = Some("Edit: Not yet implemented".to_string());
        Ok(())
    }

    pub fn copy_file(&mut self) -> Result<()> {
        self.status_message = Some("Copy: Not yet implemented".to_string());
        Ok(())
    }

    pub fn move_file(&mut self) -> Result<()> {
        self.status_message = Some("Move: Not yet implemented".to_string());
        Ok(())
    }

    pub fn make_directory(&mut self) -> Result<()> {
        self.status_message = Some("MkDir: Not yet implemented".to_string());
        Ok(())
    }

    pub fn delete_file(&mut self) -> Result<()> {
        self.status_message = Some("Delete: Not yet implemented".to_string());
        Ok(())
    }

    pub fn set_visible_rows(&mut self, rows: usize) {
        self.visible_rows = rows;
        self.left_panel.visible_rows = rows;
        self.right_panel.visible_rows = rows;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_new() {
        let app = App::new(None, None).unwrap();
        assert_eq!(app.active_panel, ActivePanel::Left);
        assert!(app.remote_connection.is_none());
    }

    #[test]
    fn test_app_with_remote_string() {
        let app = App::new(Some("user@host".to_string()), None).unwrap();
        assert_eq!(app.remote_connection, Some("user@host".to_string()));
    }

    #[test]
    fn test_toggle_panel() {
        let mut app = App::new(None, None).unwrap();
        assert_eq!(app.active_panel, ActivePanel::Left);
        app.toggle_active_panel();
        assert_eq!(app.active_panel, ActivePanel::Right);
        app.toggle_active_panel();
        assert_eq!(app.active_panel, ActivePanel::Left);
    }

    #[test]
    fn test_navigation() {
        let mut app = App::new(None, None).unwrap();
        let initial_index = app.active_panel().selected_index;
        
        app.move_selection_down();
        if app.active_panel().entries.len() > 1 {
            assert_eq!(app.active_panel().selected_index, initial_index + 1);
        }
        
        app.move_selection_up();
        assert_eq!(app.active_panel().selected_index, initial_index);
    }

    #[test]
    fn test_move_to_bounds() {
        let mut app = App::new(None, None).unwrap();
        
        app.move_to_first();
        assert_eq!(app.active_panel().selected_index, 0);
        
        app.move_to_last();
        assert_eq!(
            app.active_panel().selected_index,
            app.active_panel().entries.len().saturating_sub(1)
        );
    }
}
