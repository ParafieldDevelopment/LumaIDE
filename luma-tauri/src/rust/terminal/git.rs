use serde::Serialize;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::ipc::InvokeError;

use super::{checked_dir, invoke_error, output};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutput {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

#[tauri::command]
pub async fn git_command(cwd: String, args: Vec<String>) -> Result<CommandOutput, InvokeError> {
    let dir = checked_dir(&cwd)?;
    match args.first() {
        Some(first) if !first.starts_with('-') => {}
        _ => return Err(invoke_error("Invalid git arguments")),
    }

    let mut command = Command::new("git");
    command
        .args(&args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    run_blocking(command).await
}

fn drain<R: Read + Send + 'static>(mut reader: R) -> (Arc<Mutex<Vec<u8>>>, thread::JoinHandle<()>) {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&buffer);
    let handle = thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    if let Ok(mut bytes) = sink.lock() {
                        if bytes.len() < output::MAX_OUTPUT_BYTES {
                            bytes.extend_from_slice(&chunk[..count]);
                        }
                    }
                }
            }
        }
    });
    (buffer, handle)
}

fn snapshot(buffer: &Arc<Mutex<Vec<u8>>>) -> String {
    let bytes = buffer.lock().map(|bytes| bytes.clone()).unwrap_or_default();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn run_with_timeout(mut command: Command, timeout: Duration) -> std::io::Result<CommandOutput> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    let mut child = command.spawn()?;
    let (out_buffer, out_thread) = drain(child.stdout.take().expect("stdout is piped"));
    let (err_buffer, err_thread) = drain(child.stderr.take().expect("stderr is piped"));
    let started = Instant::now();
    let mut timed_out = false;

    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break None;
        }
        thread::sleep(Duration::from_millis(30));
    };

    if !timed_out {
        let _ = out_thread.join();
        let _ = err_thread.join();
    }

    Ok(CommandOutput {
        code: status.and_then(|status| status.code()),
        stdout: snapshot(&out_buffer),
        stderr: snapshot(&err_buffer),
        timed_out,
    })
}

async fn run_blocking(command: Command) -> Result<CommandOutput, InvokeError> {
    tauri::async_runtime::spawn_blocking(move || run_with_timeout(command, COMMAND_TIMEOUT))
        .await
        .map_err(|error| invoke_error(format!("Command task failed: {error}")))?
        .map_err(|error| invoke_error(format!("Failed to start process: {error}")))
}
