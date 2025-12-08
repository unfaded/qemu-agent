use crate::colors::Tags;
use crate::qga::QemuGuestAgent;
use anyhow::Result;
use arboard::Clipboard;
use base64::{engine::general_purpose, Engine as _};
use std::path::{Path, PathBuf};
use std::process::Command;

impl QemuGuestAgent {
    pub fn paste_clipboard(&self, source: Option<PathBuf>, dest: String) -> Result<()> {
        let dest_path = dest;

        if let Some(source_path) = source {
            let filename = source_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            let guest_dest = if Self::is_windows_path(&dest_path) {
                format!("{}\\{}", dest_path, filename)
            } else {
                format!("{}/{}", dest_path, filename)
            };

            if source_path.is_file() {
                println!("{} Uploading file: {} -> {}", Tags::upload(), source_path.display(), guest_dest);
                self.upload_file(&source_path, &guest_dest)?;
                println!("{} Upload complete", Tags::ok());
            } else if source_path.is_dir() {
                self.upload_folder(&source_path, &guest_dest)?;
            } else {
                anyhow::bail!("Source path does not exist: {}", source_path.display());
            }

            return Ok(());
        }

        println!("{} Reading clipboard...", Tags::clip());
        
        let mut content = Command::new("wl-paste")
            .args(&["--type", "x-special/gnome-copied-files"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).to_string())
                } else {
                    None
                }
            });

        if content.is_none() {
            content = Command::new("wl-paste")
                .args(&["--type", "text/uri-list"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() && !o.stdout.is_empty() {
                        Some(String::from_utf8_lossy(&o.stdout).to_string())
                    } else {
                        None
                    }
                });
        }

        if content.is_none() {
            content = Clipboard::new()
                .and_then(|mut cb| cb.get_text())
                .ok()
                .filter(|s| !s.is_empty());
        }

        let content = content.ok_or_else(|| {
            anyhow::anyhow!("Clipboard is empty or contains no file/text data")
        })?;

        println!("{} Clipboard content detected ({} bytes)", Tags::clip(), content.len());

        let lines: Vec<&str> = content.lines().collect();
        
        let is_file_list = lines.iter().any(|line| {
            line.starts_with("file://") || 
            line.starts_with('/') || 
            (line.len() > 2 && line.chars().nth(1) == Some(':'))
        });

        if is_file_list {
            println!("{} Detected file/folder paths in clipboard", Tags::clip());
            
            for line in lines {
                let line = line.trim();
                if line.is_empty() || line == "copy" || line == "cut" {
                    continue;
                }
                
                let path = if line.starts_with("file://") {
                    let uri_path = line.strip_prefix("file://").unwrap_or(line);
                    uri_path
                        .replace("%20", " ")
                        .replace("%23", "#")
                        .replace("%25", "%")
                        .replace("%26", "&")
                } else {
                    line.to_string()
                };

                let source_path = Path::new(&path);
                
                if !source_path.exists() {
                    println!("{} Non-existent path: {}", Tags::skip(), path);
                    continue;
                }

                let filename = source_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                let guest_dest = if Self::is_windows_path(&dest_path) {
                    format!("{}\\{}", dest_path, filename)
                } else {
                    format!("{}/{}", dest_path, filename)
                };

                if source_path.is_file() {
                    println!("{} Uploading file: {} -> {}", Tags::file(), path, guest_dest);
                    self.upload_file(source_path, &guest_dest)?;
                } else if source_path.is_dir() {
                    println!("{} Uploading folder: {} -> {}", Tags::folder(), path, guest_dest);
                    self.upload_folder(source_path, &guest_dest)?;
                }
            }

            println!("{} Clipboard files pasted successfully", Tags::ok());
        } else {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let filename = format!("clipboard_{}.txt", timestamp);

            let full_path = if Self::is_windows_path(&dest_path) {
                format!("{}\\{}", dest_path, filename)
            } else {
                format!("{}/{}", dest_path, filename)
            };

            println!("{} Pasting text to: {}", Tags::text(), full_path);

            let normalized_path = if Self::is_windows_path(&full_path) {
                Self::normalize_windows_path(&full_path)
            } else {
                full_path.clone()
            };

            let handle = self.guest_file_open(&normalized_path, "w+")?;

            let result = (|| -> Result<()> {
                self.guest_file_write(handle, content.as_bytes())?;
                Ok(())
            })();

            self.guest_file_close(handle)?;
            result?;

            println!("{} Clipboard pasted successfully to {}", Tags::ok(), full_path);
        }

        Ok(())
    }
}
