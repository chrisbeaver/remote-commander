use anyhow::Result;
use std::path::PathBuf;

use crate::filesystem::{FileEntry, FileSystem};

/// Represents a file panel (left or right side)
pub struct FilePanel {
    pub current_path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub visible_rows: usize,
    filesystem: Box<dyn FileSystem>,
}

impl FilePanel {
    pub fn new<F: FileSystem + 'static>(filesystem: F, path: PathBuf) -> Result<Self> {
        let entries = filesystem.list_directory(&path)?;
        
        Ok(Self {
            current_path: path,
            entries,
            selected_index: 0,
            scroll_offset: 0,
            visible_rows: 20,
            filesystem: Box::new(filesystem),
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.entries = self.filesystem.list_directory(&self.current_path)?;
        if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn change_directory(&mut self, path: &PathBuf) -> Result<()> {
        if self.filesystem.is_directory(path) {
            self.entries = self.filesystem.list_directory(path)?;
            self.current_path = path.clone();
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
        Ok(())
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }

    pub fn adjust_scroll(&mut self) {
        // Ensure selected item is visible
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.visible_rows {
            self.scroll_offset = self.selected_index - self.visible_rows + 1;
        }
    }

    pub fn visible_entries(&self) -> impl Iterator<Item = (usize, &FileEntry)> {
        self.entries
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(self.visible_rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::LocalFileSystem;
    use tempfile::TempDir;

    fn setup_test_panel() -> (TempDir, FilePanel) {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test structure
        std::fs::create_dir(temp_dir.path().join("dir_a")).unwrap();
        std::fs::create_dir(temp_dir.path().join("dir_b")).unwrap();
        std::fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();
        
        let panel = FilePanel::new(
            LocalFileSystem::new(),
            temp_dir.path().to_path_buf(),
        ).unwrap();
        
        (temp_dir, panel)
    }

    #[test]
    fn test_panel_creation() {
        let (_temp_dir, panel) = setup_test_panel();
        
        // Should have: .., dir_a, dir_b, file1.txt, file2.txt
        assert_eq!(panel.entries.len(), 5);
        assert_eq!(panel.selected_index, 0);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_selected_entry() {
        let (_temp_dir, panel) = setup_test_panel();
        
        let entry = panel.selected_entry().unwrap();
        assert_eq!(entry.name, "..");
    }

    #[test]
    fn test_change_directory() {
        let (temp_dir, mut panel) = setup_test_panel();
        
        let subdir = temp_dir.path().join("dir_a");
        panel.change_directory(&subdir).unwrap();
        
        assert_eq!(panel.current_path, subdir);
        assert_eq!(panel.selected_index, 0);
    }

    #[test]
    fn test_refresh() {
        let (temp_dir, mut panel) = setup_test_panel();
        
        // Add a new file
        std::fs::write(temp_dir.path().join("new_file.txt"), "new").unwrap();
        
        let old_count = panel.entries.len();
        panel.refresh().unwrap();
        
        assert_eq!(panel.entries.len(), old_count + 1);
    }

    #[test]
    fn test_scroll_adjustment() {
        let (_temp_dir, mut panel) = setup_test_panel();
        panel.visible_rows = 2;
        
        // Select item beyond visible area
        panel.selected_index = 3;
        panel.adjust_scroll();
        
        // Scroll should adjust to show selected item
        assert!(panel.selected_index < panel.scroll_offset + panel.visible_rows);
    }

    #[test]
    fn test_visible_entries() {
        let (_temp_dir, mut panel) = setup_test_panel();
        panel.visible_rows = 2;
        panel.scroll_offset = 1;
        
        let visible: Vec<_> = panel.visible_entries().collect();
        
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].0, 1); // Original index preserved
    }
}
