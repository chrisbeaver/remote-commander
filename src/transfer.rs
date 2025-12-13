//! File transfer operations between local and remote filesystems

use anyhow::{Context, Result};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use crate::file_panel::FilePanel;
use crate::ssh::RemoteFileSystem;

/// Buffer size for file transfers (64KB)
const BUFFER_SIZE: usize = 64 * 1024;

/// Transfer a file from the source panel to the destination panel
pub fn copy_file(
    source_panel: &FilePanel,
    dest_panel: &FilePanel,
    source_path: &Path,
    dest_path: &Path,
) -> Result<u64> {
    // Determine the transfer type based on filesystem types
    let source_is_remote = source_panel.is_remote();
    let dest_is_remote = dest_panel.is_remote();

    match (source_is_remote, dest_is_remote) {
        (false, false) => copy_local_to_local(source_path, dest_path),
        (false, true) => copy_local_to_remote(source_path, dest_path, dest_panel),
        (true, false) => copy_remote_to_local(source_path, dest_path, source_panel),
        (true, true) => copy_remote_to_remote(source_path, dest_path, source_panel, dest_panel),
    }
}

/// Copy a file locally
fn copy_local_to_local(source: &Path, dest: &Path) -> Result<u64> {
    fs::copy(source, dest).with_context(|| {
        format!(
            "Failed to copy {} to {}",
            source.display(),
            dest.display()
        )
    })
}

/// Copy a local file to a remote destination
fn copy_local_to_remote(source: &Path, dest: &Path, dest_panel: &FilePanel) -> Result<u64> {
    let sftp = dest_panel
        .get_sftp()
        .context("Destination is not a remote filesystem")?;

    let sftp_guard = sftp.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    // Read local file
    let mut local_file = fs::File::open(source)
        .with_context(|| format!("Failed to open local file: {}", source.display()))?;

    // Create remote file
    let mut remote_file = sftp_guard
        .create(dest)
        .with_context(|| format!("Failed to create remote file: {}", dest.display()))?;

    // Transfer data
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut total_bytes = 0u64;

    loop {
        let bytes_read = local_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        remote_file.write_all(&buffer[..bytes_read])?;
        total_bytes += bytes_read as u64;
    }

    Ok(total_bytes)
}

/// Copy a remote file to a local destination
fn copy_remote_to_local(source: &Path, dest: &Path, source_panel: &FilePanel) -> Result<u64> {
    let sftp = source_panel
        .get_sftp()
        .context("Source is not a remote filesystem")?;

    let sftp_guard = sftp.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    // Open remote file
    let mut remote_file = sftp_guard
        .open(source)
        .with_context(|| format!("Failed to open remote file: {}", source.display()))?;

    // Create local file
    let mut local_file = fs::File::create(dest)
        .with_context(|| format!("Failed to create local file: {}", dest.display()))?;

    // Transfer data
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut total_bytes = 0u64;

    loop {
        let bytes_read = remote_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        local_file.write_all(&buffer[..bytes_read])?;
        total_bytes += bytes_read as u64;
    }

    Ok(total_bytes)
}

/// Copy a file between two remote locations (download then upload)
fn copy_remote_to_remote(
    source: &Path,
    dest: &Path,
    source_panel: &FilePanel,
    dest_panel: &FilePanel,
) -> Result<u64> {
    let source_sftp = source_panel
        .get_sftp()
        .context("Source is not a remote filesystem")?;
    let dest_sftp = dest_panel
        .get_sftp()
        .context("Destination is not a remote filesystem")?;

    let source_guard = source_sftp.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
    let dest_guard = dest_sftp.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    // Open source file
    let mut source_file = source_guard
        .open(source)
        .with_context(|| format!("Failed to open remote source: {}", source.display()))?;

    // Create destination file
    let mut dest_file = dest_guard
        .create(dest)
        .with_context(|| format!("Failed to create remote destination: {}", dest.display()))?;

    // Transfer data
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut total_bytes = 0u64;

    loop {
        let bytes_read = source_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        dest_file.write_all(&buffer[..bytes_read])?;
        total_bytes += bytes_read as u64;
    }

    Ok(total_bytes)
}

/// Delete a file from the source panel's filesystem
pub fn delete_file(panel: &FilePanel, path: &Path) -> Result<()> {
    if panel.is_remote() {
        let sftp = panel.get_sftp().context("Not a remote filesystem")?;
        let sftp_guard = sftp.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        sftp_guard
            .unlink(path)
            .with_context(|| format!("Failed to delete remote file: {}", path.display()))?;
    } else {
        fs::remove_file(path)
            .with_context(|| format!("Failed to delete local file: {}", path.display()))?;
    }
    Ok(())
}

/// Delete a directory from the source panel's filesystem
pub fn delete_directory(panel: &FilePanel, path: &Path) -> Result<()> {
    if panel.is_remote() {
        let sftp = panel.get_sftp().context("Not a remote filesystem")?;
        let sftp_guard = sftp.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        sftp_guard
            .rmdir(path)
            .with_context(|| format!("Failed to delete remote directory: {}", path.display()))?;
    } else {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed to delete local directory: {}", path.display()))?;
    }
    Ok(())
}

/// Create a directory in the panel's filesystem
pub fn create_directory(panel: &FilePanel, path: &Path) -> Result<()> {
    if panel.is_remote() {
        let sftp = panel.get_sftp().context("Not a remote filesystem")?;
        let sftp_guard = sftp.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        sftp_guard
            .mkdir(path, 0o755)
            .with_context(|| format!("Failed to create remote directory: {}", path.display()))?;
    } else {
        fs::create_dir(path)
            .with_context(|| format!("Failed to create local directory: {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::LocalFileSystem;
    use tempfile::TempDir;

    fn create_test_panel(dir: &Path) -> FilePanel {
        FilePanel::new(LocalFileSystem::new(), dir.to_path_buf()).unwrap()
    }

    #[test]
    fn test_copy_local_to_local() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        fs::write(&source, "Hello, World!").unwrap();

        let bytes = copy_local_to_local(&source, &dest).unwrap();

        assert_eq!(bytes, 13);
        assert_eq!(fs::read_to_string(&dest).unwrap(), "Hello, World!");
    }

    #[test]
    fn test_copy_file_local_panels() {
        let source_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();

        let source_file = source_dir.path().join("test.txt");
        fs::write(&source_file, "Test content").unwrap();

        let source_panel = create_test_panel(source_dir.path());
        let dest_panel = create_test_panel(dest_dir.path());

        let dest_file = dest_dir.path().join("test.txt");
        let bytes = copy_file(&source_panel, &dest_panel, &source_file, &dest_file).unwrap();

        assert_eq!(bytes, 12);
        assert!(dest_file.exists());
        assert_eq!(fs::read_to_string(&dest_file).unwrap(), "Test content");
    }

    #[test]
    fn test_delete_local_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("to_delete.txt");
        fs::write(&file_path, "Delete me").unwrap();

        let panel = create_test_panel(temp_dir.path());
        delete_file(&panel, &file_path).unwrap();

        assert!(!file_path.exists());
    }

    #[test]
    fn test_create_local_directory() {
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("new_folder");

        let panel = create_test_panel(temp_dir.path());
        create_directory(&panel, &new_dir).unwrap();

        assert!(new_dir.is_dir());
    }

    #[test]
    fn test_delete_local_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_to_delete = temp_dir.path().join("folder_to_delete");
        fs::create_dir(&dir_to_delete).unwrap();
        fs::write(dir_to_delete.join("file.txt"), "content").unwrap();

        let panel = create_test_panel(temp_dir.path());
        delete_directory(&panel, &dir_to_delete).unwrap();

        assert!(!dir_to_delete.exists());
    }
}
