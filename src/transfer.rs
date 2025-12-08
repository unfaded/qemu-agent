use crate::colors::Tags;
use crate::qga::QemuGuestAgent;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use walkdir::WalkDir;

impl QemuGuestAgent {
    pub fn check_is_directory(&self, path: &str) -> Result<bool> {
        let pid = if Self::is_windows_path(path) {
            let ps_cmd = format!(
                r#"if (Test-Path -Path "{}" -PathType Container) {{ exit 0 }} else {{ exit 1 }}"#,
                path
            );
            self.guest_exec("powershell.exe", Some(vec!["-Command".to_string(), ps_cmd]), false)?
        } else {
            self.guest_exec("test", Some(vec!["-d".to_string(), path.to_string()]), false)?
        };

        for _ in 0..10 {
            let status = self.guest_exec_status(pid)?;
            if status.exited {
                return Ok(status.exitcode.unwrap_or(1) == 0);
            }
            thread::sleep(Duration::from_millis(100));
        }

        Ok(false)
    }

    pub fn list_directory(&self, path: &str) -> Result<Vec<String>> {
        let pid = if Self::is_windows_path(path) {
            self.guest_exec(
                "cmd.exe",
                Some(vec!["/c".to_string(), "dir".to_string(), "/b".to_string(), path.to_string()]),
                true,
            )?
        } else {
            self.guest_exec("ls", Some(vec!["-1".to_string(), path.to_string()]), true)?
        };

        for _ in 0..20 {
            let status = self.guest_exec_status(pid)?;
            if status.exited {
                if status.exitcode.unwrap_or(1) == 0 {
                    if let Some(out_data) = status.out_data {
                        let output = general_purpose::STANDARD
                            .decode(out_data)
                            .context("Failed to decode output")?;
                        let output_str = String::from_utf8_lossy(&output);
                        return Ok(output_str
                            .lines()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect());
                    }
                }
                return Ok(vec![]);
            }
            thread::sleep(Duration::from_millis(100));
        }

        Ok(vec![])
    }

    pub fn create_directory(&self, path: &str) -> Result<()> {
        let pid = if Self::is_windows_path(path) {
            self.guest_exec(
                "cmd.exe",
                Some(vec!["/c".to_string(), "mkdir".to_string(), path.to_string()]),
                false,
            )?
        } else {
            self.guest_exec("mkdir", Some(vec!["-p".to_string(), path.to_string()]), false)?
        };

        for _ in 0..10 {
            let status = self.guest_exec_status(pid)?;
            if status.exited {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }

    pub fn upload_file(&self, local_path: &Path, guest_path: &str) -> Result<()> {
        let guest_path_normalized = if Self::is_windows_path(guest_path) {
            Self::normalize_windows_path(guest_path)
        } else {
            guest_path.to_string()
        };

        let handle = self.guest_file_open(&guest_path_normalized, "w+")?;

        let result = (|| -> Result<()> {
            let mut file = fs::File::open(local_path)?;
            let mut buffer = vec![0u8; 4096];

            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                self.guest_file_write(handle, &buffer[..bytes_read])?;
            }

            Ok(())
        })();

        self.guest_file_close(handle)?;
        result
    }

    pub fn download_file(&self, guest_path: &str, local_path: &Path) -> Result<()> {
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let guest_path_normalized = if Self::is_windows_path(guest_path) {
            Self::normalize_windows_path(guest_path)
        } else {
            guest_path.to_string()
        };

        let handle = self.guest_file_open(&guest_path_normalized, "r")?;

        let result = (|| -> Result<()> {
            let mut file = fs::File::create(local_path)?;

            loop {
                let (data, eof) = self.guest_file_read(handle, 4096)?;
                if !data.is_empty() {
                    file.write_all(&data)?;
                }
                if eof {
                    break;
                }
            }

            Ok(())
        })();

        self.guest_file_close(handle)?;
        result
    }

    pub fn upload_folder(&self, local_folder: &Path, guest_folder: &str) -> Result<()> {
        println!("{} Uploading folder: {} -> {}", Tags::upload(), local_folder.display(), guest_folder);

        self.create_directory(guest_folder)?;

        let entries: Vec<_> = WalkDir::new(local_folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .collect();

        let pb = ProgressBar::new(entries.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );

        for entry in entries {
            let path = entry.path();
            let rel_path = path.strip_prefix(local_folder)?;

            if rel_path.as_os_str().is_empty() {
                continue;
            }

            let guest_path = if Self::is_windows_path(guest_folder) {
                format!("{}\\{}", guest_folder, rel_path.to_string_lossy().replace('/', "\\"))
            } else {
                format!("{}/{}", guest_folder, rel_path.to_string_lossy())
            };

            if path.is_dir() {
                self.create_directory(&guest_path)?;
            } else {
                pb.set_message(format!("Uploading {}", rel_path.display()));
                self.upload_file(path, &guest_path)?;
            }

            pb.inc(1);
        }

        pb.finish_with_message(format!("{} Upload complete", Tags::ok()));
        Ok(())
    }

    pub fn download_folder(&self, guest_folder: &str, local_folder: &Path) -> Result<()> {
        println!("{} Downloading folder: {} -> {}", Tags::download(), guest_folder, local_folder.display());

        fs::create_dir_all(local_folder)?;

        let items = self.list_directory(guest_folder)?;

        for item in items {
            let guest_item_path = if Self::is_windows_path(guest_folder) {
                format!("{}\\{}", guest_folder, item)
            } else {
                format!("{}/{}", guest_folder, item)
            };

            let local_item_path = local_folder.join(&item);

            if self.check_is_directory(&guest_item_path)? {
                self.download_folder(&guest_item_path, &local_item_path)?;
            } else {
                println!("{} Downloading {}", Tags::file(), item);
                self.download_file(&guest_item_path, &local_item_path)?;
            }
        }

        println!("{} Folder downloaded successfully", Tags::ok());
        Ok(())
    }
}
