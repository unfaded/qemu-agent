use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
mod colors;
mod cli;
mod qga;
mod transfer;
mod clipboard;

use cli::{Cli, Commands};
use colors::Tags;
use qga::QemuGuestAgent;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let agent = QemuGuestAgent::new(cli.vm_name);

    // Set up Ctrl+C handler for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
        println!("\n{} Cleaning up...", Tags::interrupt());
    }).context("Error setting Ctrl-C handler")?;

    match cli.command {
        Commands::Upload { source, dest } => {
            if source.is_file() {
                println!("{} Uploading file: {} -> {}", Tags::upload(), source.display(), dest);
                agent.upload_file(&source, &dest)?;
                println!("{} Upload complete", Tags::ok());
            } else if source.is_dir() {
                agent.upload_folder(&source, &dest)?;
            } else {
                anyhow::bail!("Source path does not exist: {}", source.display());
            }
        }
        Commands::Download { source, dest } => {
            if agent.check_is_directory(&source)? {
                agent.download_folder(&source, &dest)?;
            } else {
                println!("{} Downloading file: {} -> {}", Tags::download(), source, dest.display());
                agent.download_file(&source, &dest)?;
                println!("{} Download complete", Tags::ok());
            }
        }
        Commands::Paste { source, dest } => {
            agent.paste_clipboard(source, dest)?;
        }
        Commands::Exec { command, args } => {
            println!("{} Executing: {} {:?}", Tags::exec(), command, args);
            let pid = agent.guest_exec(&command, Some(args), true)?;

            // Wait for completion
            for _ in 0..30 {
                let status = agent.guest_exec_status(pid)?;
                if status.exited {
                    println!("Exit code: {}", status.exitcode.unwrap_or(-1));

                    if let Some(out_data) = status.out_data {
                        let output = general_purpose::STANDARD.decode(out_data)?;
                        println!("Output:\n{}", String::from_utf8_lossy(&output));
                    }

                    if let Some(err_data) = status.err_data {
                        let output = general_purpose::STANDARD.decode(err_data)?;
                        eprintln!("Error:\n{}", String::from_utf8_lossy(&output));
                    }

                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    }

    Ok(())
}
