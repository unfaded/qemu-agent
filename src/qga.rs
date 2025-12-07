use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Command;

#[derive(Serialize, Deserialize, Debug)]
struct QemuCommand {
    execute: String,
    arguments: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct QemuResponse {
    #[serde(rename = "return")]
    return_value: serde_json::Value,
}

#[derive(Deserialize, Debug)]
pub struct ExecStatus {
    pub exited: bool,
    pub exitcode: Option<i32>,
    #[serde(rename = "out-data")]
    pub out_data: Option<String>,
    #[serde(rename = "err-data")]
    pub err_data: Option<String>,
}

pub struct QemuGuestAgent {
    pub vm_name: String,
}

impl QemuGuestAgent {
    pub fn new(vm_name: String) -> Self {
        Self { vm_name }
    }

    pub fn run_command(&self, command: &str, arguments: serde_json::Value) -> Result<serde_json::Value> {
        let cmd = QemuCommand {
            execute: command.to_string(),
            arguments,
        };

        let json_cmd = serde_json::to_string(&cmd)?;

        let output = Command::new("sudo")
            .args(&["virsh", "qemu-agent-command", &self.vm_name, &json_cmd])
            .output()
            .context("Failed to execute virsh command")?;

        if !output.status.success() {
            anyhow::bail!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let response: QemuResponse = serde_json::from_slice(&output.stdout)
            .context("Failed to parse QEMU response")?;

        Ok(response.return_value)
    }

    pub fn guest_file_open(&self, path: &str, mode: &str) -> Result<i64> {
        let result = self.run_command(
            "guest-file-open",
            json!({
                "path": path,
                "mode": mode
            }),
        )?;

        result
            .as_i64()
            .context("Failed to get file handle")
    }

    pub fn guest_file_close(&self, handle: i64) -> Result<()> {
        self.run_command("guest-file-close", json!({ "handle": handle }))?;
        Ok(())
    }

    pub fn guest_file_write(&self, handle: i64, data: &[u8]) -> Result<usize> {
        let b64_data = general_purpose::STANDARD.encode(data);
        let result = self.run_command(
            "guest-file-write",
            json!({
                "handle": handle,
                "buf-b64": b64_data
            }),
        )?;

        result["count"]
            .as_u64()
            .map(|v| v as usize)
            .context("Failed to get write count")
    }

    pub fn guest_file_read(&self, handle: i64, count: usize) -> Result<(Vec<u8>, bool)> {
        let result = self.run_command(
            "guest-file-read",
            json!({
                "handle": handle,
                "count": count
            }),
        )?;

        let b64_data = result["buf-b64"]
            .as_str()
            .context("Failed to get buf-b64")?;
        let data = general_purpose::STANDARD
            .decode(b64_data)
            .context("Failed to decode base64")?;
        let eof = result["eof"].as_bool().unwrap_or(false);

        Ok((data, eof))
    }

    pub fn guest_exec(&self, path: &str, args: Option<Vec<String>>, capture_output: bool) -> Result<i64> {
        let mut arguments = json!({ "path": path });

        if let Some(args) = args {
            arguments["arg"] = json!(args);
        }

        if capture_output {
            arguments["capture-output"] = json!(true);
        }

        let result = self.run_command("guest-exec", arguments)?;

        result["pid"]
            .as_i64()
            .context("Failed to get PID")
    }

    pub fn guest_exec_status(&self, pid: i64) -> Result<ExecStatus> {
        let result = self.run_command("guest-exec-status", json!({ "pid": pid }))?;
        serde_json::from_value(result).context("Failed to parse exec status")
    }

    pub fn is_windows_path(path: &str) -> bool {
        path.len() > 1 && path.chars().nth(1) == Some(':')
    }

    pub fn normalize_windows_path(path: &str) -> String {
        path.replace('/', "\\").replace('\\', "\\\\")
    }
}

pub use serde_json::json;
