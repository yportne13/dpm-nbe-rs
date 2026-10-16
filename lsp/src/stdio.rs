//! `Content-Length` framing around real `stdin`/`stdout`.
//!
//! The Language Server Protocol frames every message as
//!
//! ```text
//! Content-Length: <bytes>\r\n
//! \r\n
//! <utf-8 JSON>
//! ```
//!
//! This module is native-only: the browser build hands already-framed message
//! strings to [`crate::protocol::Server::handle_message`] directly.

use crate::Server;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

/// Refuse absurd frames instead of trying to allocate them.
const MAX_FRAME: usize = 64 * 1024 * 1024;

/// Serve LSP on the process's `stdin`/`stdout` until `exit` or end of input.
pub fn serve() -> io::Result<ExitCode> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    serve_io(&mut input, &mut output)
}

/// The testable core of [`serve`].
pub fn serve_io<R: BufRead, W: Write>(input: &mut R, output: &mut W) -> io::Result<ExitCode> {
    let mut server = Server::new();
    loop {
        let frame = match read_frame(input)? {
            Some(f) => f,
            None => break,
        };
        let text = String::from_utf8_lossy(&frame).into_owned();
        for message in server.handle_message(&text) {
            write_frame(output, message.as_bytes())?;
        }
        output.flush()?;
        if server.should_exit() {
            break;
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Read one framed message, or `None` at end of input.
pub fn read_frame<R: BufRead>(input: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut content_length: Option<usize> = None;
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = input.read_until(b'\n', &mut line)?;
        if read == 0 {
            // Clean EOF before a header, or a truncated header: either way stop.
            return Ok(None);
        }
        let header = String::from_utf8_lossy(&line);
        let header = header.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            break;
        }
        if let Some(value) = header
            .split_once(':')
            .filter(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
            .map(|(_, value)| value.trim())
        {
            content_length = value.parse::<usize>().ok();
        }
    }

    let len = match content_length {
        Some(len) if len <= MAX_FRAME => len,
        Some(len) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("message of {len} bytes exceeds the {MAX_FRAME} byte limit"),
            ))
        }
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "message is missing its Content-Length header",
            ))
        }
    };

    let mut body = vec![0u8; len];
    input.read_exact(&mut body)?;
    Ok(Some(body))
}

/// Write one framed message.
pub fn write_frame<W: Write>(output: &mut W, body: &[u8]) -> io::Result<()> {
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(body)
}
