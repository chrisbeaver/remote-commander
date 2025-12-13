use anyhow::Result;
use chrono::{DateTime, Local};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Represents a file or directory entry
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<DateTime<Local>>,
    pub permissions: u32,
}

impl FileEntry {
    pub fn format_size(&self) -> String {
        if self.is_dir {
            "<DIR>".to_string()
        } else {
            format_file_size(self.size)
        }
    }

    pub fn format_date(&self) -> String {
        self.modified
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "".to_string())
    }

    pub fn format_permissions(&self) -> String {
        format_unix_permissions(self.permissions)
    }
}

fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{}B", size)
    }
}

fn format_unix_permissions(mode: u32) -> String {
    let user = format_rwx((mode >> 6) & 0o7);
    let group = format_rwx((mode >> 3) & 0o7);
    let other = format_rwx(mode & 0o7);
    format!("{}{}{}", user, group, other)
}

fn format_rwx(bits: u32) -> String {
    let r = if bits & 0o4 != 0 { 'r' } else { '-' };
    let w = if bits & 0o2 != 0 { 'w' } else { '-' };
    let x = if bits & 0o1 != 0 { 'x' } else { '-' };
    format!("{}{}{}", r, w, x)
}

/// Trait for file system operations (enables local/remote abstraction)
pub trait FileSystem {
    fn list_directory(&self, path: &Path) -> Result<Vec<FileEntry>>;
    fn is_directory(&self, path: &Path) -> bool;
    fn exists(&self, path: &Path) -> bool;
}

/// Local file system implementation
#[derive(Debug, Clone)]
pub struct LocalFileSystem;

impl LocalFileSystem {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem for LocalFileSystem {
    fn list_directory(&self, path: &Path) -> Result<Vec<FileEntry>> {
        let mut entries = Vec::new();

        // Add parent directory entry if not at root
        if path.parent().is_some() {
            entries.push(FileEntry {
                name: "..".to_string(),
                path: path.parent().unwrap().to_path_buf(),
                is_dir: true,
                size: 0,
                modified: None,
                permissions: 0o755,
            });
        }

        let read_dir = fs::read_dir(path)?;
        
        for entry in read_dir.flatten() {
            let path = entry.path();
            let metadata = entry.metadata().ok();
            
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .map(|t| DateTime::<Local>::from(t));
            let permissions = metadata
                .as_ref()
                .map(|m| m.permissions().mode() & 0o777)
                .unwrap_or(0);

            entries.push(FileEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                path,
                is_dir,
                size,
                modified,
                permissions,
            });
        }

        // Sort: directories first, then by name
        entries.sort_by(|a, b| {
            if a.name == ".." {
                std::cmp::Ordering::Less
            } else if b.name == ".." {
                std::cmp::Ordering::Greater
            } else if a.is_dir && !b.is_dir {
                std::cmp::Ordering::Less
            } else if !a.is_dir && b.is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        Ok(entries)
    }

    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0B");
        assert_eq!(format_file_size(512), "512B");
        assert_eq!(format_file_size(1024), "1.0K");
        assert_eq!(format_file_size(1536), "1.5K");
        assert_eq!(format_file_size(1048576), "1.0M");
        assert_eq!(format_file_size(1073741824), "1.0G");
    }

    #[test]
    fn test_format_permissions() {
        assert_eq!(format_unix_permissions(0o755), "rwxr-xr-x");
        assert_eq!(format_unix_permissions(0o644), "rw-r--r--");
        assert_eq!(format_unix_permissions(0o777), "rwxrwxrwx");
        assert_eq!(format_unix_permissions(0o000), "---------");
    }

    #[test]
    fn test_local_filesystem_list_directory() {
        let temp_dir = TempDir::new().unwrap();
        let fs = LocalFileSystem::new();
        
        // Create some test files and directories
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        std::fs::write(temp_dir.path().join("file.txt"), "test").unwrap();
        
        let entries = fs.list_directory(temp_dir.path()).unwrap();
        
        // Should have parent (..), subdir, and file.txt
        assert!(entries.len() >= 3);
        
        // Parent should be first
        assert_eq!(entries[0].name, "..");
        
        // Directory should come before file
        let subdir_pos = entries.iter().position(|e| e.name == "subdir").unwrap();
        let file_pos = entries.iter().position(|e| e.name == "file.txt").unwrap();
        assert!(subdir_pos < file_pos);
    }

    #[test]
    fn test_file_entry_format() {
        let entry = FileEntry {
            name: "test.txt".to_string(),
            path: PathBuf::from("/tmp/test.txt"),
            is_dir: false,
            size: 2048,
            modified: None,
            permissions: 0o644,
        };

        assert_eq!(entry.format_size(), "2.0K");
        assert_eq!(entry.format_permissions(), "rw-r--r--");
    }

    #[test]
    fn test_dir_entry_format() {
        let entry = FileEntry {
            name: "mydir".to_string(),
            path: PathBuf::from("/tmp/mydir"),
            is_dir: true,
            size: 4096,
            modified: None,
            permissions: 0o755,
        };

        assert_eq!(entry.format_size(), "<DIR>");
    }
}
