use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, TimeZone};
use ssh2::{Session, Sftp};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::filesystem::{FileEntry, FileSystem};

/// Parsed SSH connection string
#[derive(Debug, Clone)]
pub struct SshConnectionInfo {
    pub username: String,
    pub hostname: String,
    pub port: u16,
}

impl SshConnectionInfo {
    /// Parse a connection string like "user@hostname" or "user@hostname:port"
    pub fn parse(connection_string: &str) -> Result<Self> {
        let (user_host, port) = if connection_string.contains(':') {
            let parts: Vec<&str> = connection_string.rsplitn(2, ':').collect();
            let port: u16 = parts[0].parse().context("Invalid port number")?;
            (parts[1], port)
        } else {
            (connection_string, 22)
        };

        let parts: Vec<&str> = user_host.splitn(2, '@').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid connection string. Expected format: user@hostname[:port]"
            ));
        }

        Ok(Self {
            username: parts[0].to_string(),
            hostname: parts[1].to_string(),
            port,
        })
    }
}

/// SSH connection manager
pub struct SshConnection {
    session: Session,
    sftp: Sftp,
    pub info: SshConnectionInfo,
    pub home_dir: PathBuf,
}

impl SshConnection {
    /// Establish an SSH connection
    pub fn connect(info: SshConnectionInfo, password: Option<&str>) -> Result<Self> {
        let addr = format!("{}:{}", info.hostname, info.port);
        let tcp = TcpStream::connect(&addr)
            .with_context(|| format!("Failed to connect to {}", addr))?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // Try SSH key authentication first
        let auth_success = Self::try_key_auth(&session, &info.username)
            .unwrap_or(false);

        if !auth_success {
            // Fall back to password authentication
            if let Some(pwd) = password {
                session
                    .userauth_password(&info.username, pwd)
                    .context("Password authentication failed")?;
            } else {
                return Err(anyhow!(
                    "SSH key authentication failed and no password provided"
                ));
            }
        }

        if !session.authenticated() {
            return Err(anyhow!("Authentication failed"));
        }

        let sftp = session.sftp()?;
        
        // Get user's home directory
        let home_dir = Self::get_home_directory(&session, &info.username)?;

        Ok(Self {
            session,
            sftp,
            info,
            home_dir,
        })
    }

    /// Try to authenticate using SSH keys
    fn try_key_auth(session: &Session, username: &str) -> Result<bool> {
        // Try SSH agent first
        if let Ok(mut agent) = session.agent() {
            if agent.connect().is_ok() {
                agent.list_identities().ok();
                for identity in agent.identities().unwrap_or_default() {
                    if agent.userauth(username, &identity).is_ok() {
                        return Ok(true);
                    }
                }
            }
        }

        // Try default key locations
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Cannot find home directory"))?;
        let ssh_dir = home.join(".ssh");

        let key_files = ["id_ed25519", "id_rsa", "id_ecdsa", "id_dsa"];
        
        for key_name in &key_files {
            let private_key = ssh_dir.join(key_name);
            let public_key = ssh_dir.join(format!("{}.pub", key_name));

            if private_key.exists() {
                // Try without passphrase first
                if session
                    .userauth_pubkey_file(username, Some(&public_key), &private_key, None)
                    .is_ok()
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get the user's home directory on the remote system
    fn get_home_directory(session: &Session, username: &str) -> Result<PathBuf> {
        let mut channel = session.channel_session()?;
        channel.exec("echo $HOME")?;
        
        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;
        
        let home = output.trim();
        if home.is_empty() {
            // Fallback to /home/username
            Ok(PathBuf::from(format!("/home/{}", username)))
        } else {
            Ok(PathBuf::from(home))
        }
    }

    /// Get the SFTP handle
    pub fn sftp(&self) -> &Sftp {
        &self.sftp
    }
}

/// Remote file system implementation using SFTP
pub struct RemoteFileSystem {
    sftp: Arc<Mutex<Sftp>>,
}

impl RemoteFileSystem {
    pub fn new(connection: &SshConnection) -> Self {
        // We need to clone the Sftp handle - but ssh2 doesn't allow that easily
        // So we'll use Arc<Mutex> for thread safety
        Self {
            sftp: Arc::new(Mutex::new(connection.session.sftp().unwrap())),
        }
    }

    pub fn from_sftp(sftp: Sftp) -> Self {
        Self {
            sftp: Arc::new(Mutex::new(sftp)),
        }
    }
}

impl FileSystem for RemoteFileSystem {
    fn list_directory(&self, path: &Path) -> Result<Vec<FileEntry>> {
        let sftp = self.sftp.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        
        let mut entries = Vec::new();

        // Add parent directory entry if not at root
        if path.parent().is_some() && path != Path::new("/") {
            entries.push(FileEntry {
                name: "..".to_string(),
                path: path.parent().unwrap().to_path_buf(),
                is_dir: true,
                size: 0,
                modified: None,
                permissions: 0o755,
            });
        }

        let dir_entries = sftp.readdir(path)
            .with_context(|| format!("Failed to read directory: {}", path.display()))?;

        for (file_path, stat) in dir_entries {
            let name = file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Skip hidden . entry
            if name == "." {
                continue;
            }

            let is_dir = stat.is_dir();
            let size = stat.size.unwrap_or(0);
            let modified = stat.mtime.map(|t| {
                Local.timestamp_opt(t as i64, 0).single().unwrap_or_else(Local::now)
            });
            let permissions = stat.perm.unwrap_or(0) & 0o777;

            entries.push(FileEntry {
                name,
                path: file_path,
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
        let sftp = match self.sftp.lock() {
            Ok(s) => s,
            Err(_) => return false,
        };
        
        sftp.stat(path)
            .map(|stat| stat.is_dir())
            .unwrap_or(false)
    }

    fn exists(&self, path: &Path) -> bool {
        let sftp = match self.sftp.lock() {
            Ok(s) => s,
            Err(_) => return false,
        };
        
        sftp.stat(path).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_connection_string_simple() {
        let info = SshConnectionInfo::parse("user@hostname").unwrap();
        assert_eq!(info.username, "user");
        assert_eq!(info.hostname, "hostname");
        assert_eq!(info.port, 22);
    }

    #[test]
    fn test_parse_connection_string_with_port() {
        let info = SshConnectionInfo::parse("admin@server.com:2222").unwrap();
        assert_eq!(info.username, "admin");
        assert_eq!(info.hostname, "server.com");
        assert_eq!(info.port, 2222);
    }

    #[test]
    fn test_parse_connection_string_invalid() {
        let result = SshConnectionInfo::parse("hostname");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_connection_string_invalid_port() {
        let result = SshConnectionInfo::parse("user@host:notaport");
        assert!(result.is_err());
    }
}
