use anyhow::Result;
use std::path::PathBuf;

use crate::file_panel::FilePanel;
use crate::filesystem::{FileEntry, LocalFileSystem};
use crate::shell::{LocalShell, RemoteShell, ShellType};
use crate::ssh::{RemoteFileSystem, SshConnection};
use crate::transfer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub enum ConfirmationAction {
    Copy { source: FileEntry, dest_path: PathBuf },
    Move { source: FileEntry, dest_path: PathBuf },
    Delete { entry: FileEntry },
}

pub struct App {
    pub left_panel: FilePanel,
    pub right_panel: FilePanel,
    pub active_panel: ActivePanel,
    pub remote_connection: Option<String>,
    pub show_help: bool,
    pub show_terminal: bool,
    pub terminal_input_mode: bool,
    pub confirmation_dialog: Option<ConfirmationAction>,
    pub status_message: Option<String>,
    pub visible_rows: usize,
    pub left_shell: Option<ShellType>,
    pub right_shell: Option<ShellType>,
}

impl App {
    pub fn new(remote_connection: Option<String>, ssh_connection: Option<SshConnection>) -> Result<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        
        let left_panel = FilePanel::new(LocalFileSystem::new(), home.clone())?;
        
        // Initialize local shell for left panel
        let left_shell = LocalShell::new().ok().map(ShellType::Local);
        
        // If SSH connection provided, use remote filesystem for right panel
        let (right_panel, right_shell) = if let Some(ref ssh_conn) = ssh_connection {
            let remote_fs = RemoteFileSystem::new(ssh_conn);
            let sftp_handle = remote_fs.sftp_handle();
            let remote_home = ssh_conn.home_dir.clone();
            let panel = FilePanel::new_remote(remote_fs, remote_home, sftp_handle)?;
            
            // Create remote shell using the SSH session
            let remote_shell = RemoteShell::new(&ssh_conn.session).ok().map(ShellType::Remote);
            
            (panel, remote_shell)
        } else {
            let panel = FilePanel::new(LocalFileSystem::new(), home)?;
            let local_shell = LocalShell::new().ok().map(ShellType::Local);
            (panel, local_shell)
        };

        Ok(Self {
            left_panel,
            right_panel,
            active_panel: ActivePanel::Left,
            remote_connection,
            show_help: false,
            show_terminal: false,
            terminal_input_mode: false,
            confirmation_dialog: None,
            status_message: None,
            visible_rows: 20, // Will be updated by UI
            left_shell,
            right_shell,
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

    pub fn toggle_terminal(&mut self) {
        self.show_terminal = !self.show_terminal;
        // Don't clear output - let the user see the shell prompt
        self.status_message = Some(format!(
            "Terminal: {}",
            if self.show_terminal { "ON" } else { "OFF" }
        ));
    }

    pub fn clear_terminal_output(&mut self) {
        let shell = match self.active_panel {
            ActivePanel::Left => &mut self.left_shell,
            ActivePanel::Right => &mut self.right_shell,
        };
        
        if let Some(shell) = shell {
            shell.clear_output();
        }
    }

    pub fn enter_terminal_input_mode(&mut self) {
        self.terminal_input_mode = true;
        // Don't send clear command - just enter input mode
        self.status_message = Some("Terminal input mode (Tab/Esc to exit)".to_string());
    }

    pub fn exit_terminal_input_mode(&mut self) {
        self.terminal_input_mode = false;
        self.status_message = Some("Navigation mode (Tab=switch panel, Enter=terminal input)".to_string());
    }

    pub fn send_to_shell(&mut self, data: &[u8]) -> Result<()> {
        let shell = match self.active_panel {
            ActivePanel::Left => &mut self.left_shell,
            ActivePanel::Right => &mut self.right_shell,
        };
        
        if let Some(shell) = shell {
            shell.write_input(data)?;
        }
        Ok(())
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
        // Get source entry from active panel
        let source_entry = match self.active_panel().selected_entry() {
            Some(entry) => entry.clone(),
            None => {
                self.status_message = Some("No file selected".to_string());
                return Ok(());
            }
        };

        // Skip ".." entry
        if source_entry.name == ".." {
            self.status_message = Some("Cannot copy parent directory reference".to_string());
            return Ok(());
        }

        // Directories not yet supported
        if source_entry.is_dir {
            self.status_message = Some("Directory copy not yet implemented".to_string());
            return Ok(());
        }

        // Get destination path (inactive panel's current directory + filename)
        let dest_path = self.inactive_panel().current_path.join(&source_entry.name);

        // Show confirmation dialog
        self.confirmation_dialog = Some(ConfirmationAction::Copy {
            source: source_entry,
            dest_path,
        });

        Ok(())
    }

    pub fn move_file(&mut self) -> Result<()> {
        // Get source entry from active panel
        let source_entry = match self.active_panel().selected_entry() {
            Some(entry) => entry.clone(),
            None => {
                self.status_message = Some("No file selected".to_string());
                return Ok(());
            }
        };

        // Skip ".." entry
        if source_entry.name == ".." {
            self.status_message = Some("Cannot move parent directory reference".to_string());
            return Ok(());
        }

        // Directories not yet supported
        if source_entry.is_dir {
            self.status_message = Some("Directory move not yet implemented".to_string());
            return Ok(());
        }

        // Get destination path
        let dest_path = self.inactive_panel().current_path.join(&source_entry.name);

        // Show confirmation dialog
        self.confirmation_dialog = Some(ConfirmationAction::Move {
            source: source_entry,
            dest_path,
        });

        Ok(())
    }

    pub fn make_directory(&mut self) -> Result<()> {
        self.status_message = Some("MkDir: Not yet implemented (needs input dialog)".to_string());
        Ok(())
    }

    pub fn delete_file(&mut self) -> Result<()> {
        // Get selected entry from active panel
        let entry = match self.active_panel().selected_entry() {
            Some(entry) => entry.clone(),
            None => {
                self.status_message = Some("No file selected".to_string());
                return Ok(());
            }
        };

        // Skip ".." entry
        if entry.name == ".." {
            self.status_message = Some("Cannot delete parent directory reference".to_string());
            return Ok(());
        }

        // Show confirmation dialog
        self.confirmation_dialog = Some(ConfirmationAction::Delete { entry });

        Ok(())
    }

    pub fn set_visible_rows(&mut self, rows: usize) {
        self.visible_rows = rows;
        self.left_panel.visible_rows = rows;
        self.right_panel.visible_rows = rows;
    }

    pub fn confirm_action(&mut self) -> Result<()> {
        if let Some(action) = self.confirmation_dialog.take() {
            match action {
                ConfirmationAction::Copy { source, dest_path } => {
                    // Perform copy based on active panel
                    let result = match self.active_panel {
                        ActivePanel::Left => {
                            transfer::copy_file(&self.left_panel, &self.right_panel, &source.path, &dest_path)
                        }
                        ActivePanel::Right => {
                            transfer::copy_file(&self.right_panel, &self.left_panel, &source.path, &dest_path)
                        }
                    };

                    match result {
                        Ok(bytes) => {
                            self.status_message = Some(format!(
                                "Copied {} ({} bytes)",
                                source.name,
                                bytes
                            ));
                            // Refresh destination panel
                            match self.active_panel {
                                ActivePanel::Left => self.right_panel.refresh()?,
                                ActivePanel::Right => self.left_panel.refresh()?,
                            }
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Copy failed: {}", e));
                        }
                    }
                }
                ConfirmationAction::Move { source, dest_path } => {
                    // Perform copy then delete (move = copy + delete source)
                    let copy_result = match self.active_panel {
                        ActivePanel::Left => {
                            transfer::copy_file(&self.left_panel, &self.right_panel, &source.path, &dest_path)
                        }
                        ActivePanel::Right => {
                            transfer::copy_file(&self.right_panel, &self.left_panel, &source.path, &dest_path)
                        }
                    };

                    match copy_result {
                        Ok(bytes) => {
                            // Delete source file
                            let delete_result = match self.active_panel {
                                ActivePanel::Left => transfer::delete_file(&self.left_panel, &source.path),
                                ActivePanel::Right => transfer::delete_file(&self.right_panel, &source.path),
                            };

                            match delete_result {
                                Ok(()) => {
                                    self.status_message = Some(format!(
                                        "Moved {} ({} bytes)",
                                        source.name,
                                        bytes
                                    ));
                                    // Refresh both panels
                                    self.left_panel.refresh()?;
                                    self.right_panel.refresh()?;
                                }
                                Err(e) => {
                                    self.status_message = Some(format!(
                                        "Copied but failed to delete source: {}",
                                        e
                                    ));
                                    // Still refresh destination
                                    match self.active_panel {
                                        ActivePanel::Left => self.right_panel.refresh()?,
                                        ActivePanel::Right => self.left_panel.refresh()?,
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Move failed: {}", e));
                        }
                    }
                }
                ConfirmationAction::Delete { entry } => {
                    // Delete based on type
                    let result = if entry.is_dir {
                        match self.active_panel {
                            ActivePanel::Left => transfer::delete_directory(&self.left_panel, &entry.path),
                            ActivePanel::Right => transfer::delete_directory(&self.right_panel, &entry.path),
                        }
                    } else {
                        match self.active_panel {
                            ActivePanel::Left => transfer::delete_file(&self.left_panel, &entry.path),
                            ActivePanel::Right => transfer::delete_file(&self.right_panel, &entry.path),
                        }
                    };

                    match result {
                        Ok(()) => {
                            self.status_message = Some(format!("Deleted {}", entry.name));
                            // Refresh active panel
                            self.active_panel_mut().refresh()?;
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Delete failed: {}", e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn cancel_confirmation(&mut self) {
        self.confirmation_dialog = None;
        self.status_message = Some("Cancelled".to_string());
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
