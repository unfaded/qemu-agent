use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "qemu-ga")]
#[command(about = "QEMU Guest Agent CLI tool", long_about = None)]
pub struct Cli {
    /// VM name
    pub vm_name: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Upload file or folder to guest
    Upload {
        /// Source path on host
        source: PathBuf,
        /// Destination path on guest
        dest: String,
    },
    /// Download file or folder from guest
    Download {
        /// Source path on guest
        source: String,
        /// Destination path on host
        dest: PathBuf,
    },
    /// Paste clipboard content to guest (auto-detects files/text and destination)
    Paste {
        /// Override clipboard with manual source path (advanced)
        #[arg(short, long, hide = true)]
        source: Option<PathBuf>,
        /// Destination path on guest (required)
        #[arg(short, long)]
        dest: String,
    },
    /// Execute command in guest
    Exec {
        /// Command to execute
        command: String,
        /// Arguments
        args: Vec<String>,
    },
}
