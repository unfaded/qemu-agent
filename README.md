# qemu-agent - QEMU Guest Agent CLI Tool

Rust-based CLI tool for interacting with QEMU/KVM virtual machines through the QEMU Guest Agent protocol. Built for operators who need predictable file transfer, clipboard paste, and remote execution against Windows or Linux guests but primarily with Windows guests in mind.

## Features

- Bidirectional file/folder transfer with auto-detection
- Clipboard paste to an explicit destination you provide
- Progress bars for folder operations
- Cross-platform support for Windows and Linux guests
- Command execution with output capture  

## Installation

```bash
cargo install --path .
```

## Usage

### Upload files/folders to guest

```bash
# Upload a file
qemu-ga <VM_NAME> upload /path/to/file.txt "C:\\Users\\user\\Documents\\file.txt"

# Upload a folder (recursive, with progress bar)
qemu-ga <VM_NAME> upload /path/to/folder "C:\\Users\\user\\Documents\\folder"
```

### Download files/folders from guest

```bash
# Download a file
qemu-ga <VM_NAME> download "C:\\Users\\user\\Documents\\file.txt" ./file.txt

# Download a folder (recursive)
qemu-ga <VM_NAME> download "C:\\Users\\user\\Documents\\folder" ./folder
```

### Clipboard Paste (destination required)

```bash
# Copy files/folders or text locally, then paste to a guest path
qemu-ga <VM_NAME> paste --dest "C:\\Users\\user\\Desktop"

# Paste to Linux guests works the same, just use a POSIX path
qemu-ga <VM_NAME> paste --dest "/home/user/Desktop"
```

How it works:
- Reads clipboard content (file URIs, text/uri-list, or plain text)
- Pastes files/folders or text to the destination you pass with `--dest`

### Execute commands in guest

```bash
# Execute a command and see output
qemu-ga <VM_NAME> exec cmd.exe /c dir

# Execute PowerShell commands
qemu-ga <VM_NAME> exec powershell.exe -Command "Get-Process | Select -First 5"
```

## Examples

### Quick folder transfer
```bash
qemu-ga Win11-Example upload ./my-project "C:\\Users\\user\\Desktop\\my-project"
```

### Download and verify
```bash
qemu-ga Win11-Example download "C:\\Users\\user\\Documents\\project" ./project-backup
```

## Technical Details

Performance:
- 4KB chunk size for file transfers
- Real-time progress bars via `indicatif`
- Compiled Rust binary for fast execution

## Logging

Console output uses bracketed tags (e.g., `[UPLOAD]`, `[DOWNLOAD]`, `[OK]`) with ANSI colors for quick scanning. Errors are returned with context to aid troubleshooting.

## Troubleshooting

### "guest agent command failed"
- Ensure [qemu-guest-agent](https://wiki.qemu.org/Features/GuestAgent) is installed in the guest VM

### Permission errors
- The tool uses `sudo virsh` - ensure your user can run sudo and virsh is installed

### Operational tips
- Validate guest free space before large uploads; transfers do not resume.
- Run from a shell with access to `wl-paste` when using clipboard features.