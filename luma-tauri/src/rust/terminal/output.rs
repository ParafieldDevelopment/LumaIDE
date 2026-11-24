use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;

pub const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Default)]
pub struct TerminalOutputBuffer {
    pub bytes: Vec<u8>,
    pub base_offset: usize,
    pub next_offset: usize,
}

/// Continuously collect PTY output, trimming old bytes once the buffer is full.
pub fn drain_terminal<R: Read + Send + 'static>(
    mut reader: R,
    buffer: Arc<Mutex<TerminalOutputBuffer>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    let Ok(mut output) = buffer.lock() else {
                        break;
                    };
                    output.next_offset = output.next_offset.saturating_add(count);
                    output.bytes.extend_from_slice(&chunk[..count]);
                    if output.bytes.len() > MAX_OUTPUT_BYTES {
                        let trim = output.bytes.len() - MAX_OUTPUT_BYTES;
                        output.bytes.drain(..trim);
                        output.base_offset = output.base_offset.saturating_add(trim);
                    }
                }
            }
        }
    })
}
