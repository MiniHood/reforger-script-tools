use serde_json::Value;
use std::io::BufRead;

const MAX_LSP_HEADER_LINE_BYTES: usize = 8 * 1024;
const MAX_LSP_HEADER_BYTES: usize = 32 * 1024;
const MAX_LSP_BODY_BYTES: usize = 16 * 1024 * 1024;

pub(crate) fn read_message<R: BufRead>(reader: &mut R) -> Result<Option<Value>, String> {
    let mut content_length = None;
    let mut header_bytes = 0;
    loop {
        let Some(line) = read_header_line(reader)? else {
            return Ok(None);
        };
        header_bytes += line.len();
        if header_bytes > MAX_LSP_HEADER_BYTES {
            return Err("LSP headers exceed the configured limit".to_string());
        }
        let line = std::str::from_utf8(&line)
            .map_err(|error| format!("Invalid LSP header encoding: {error}"))?;
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|error| format!("Invalid Content-Length: {error}"))?,
            );
        }
    }

    let Some(content_length) = content_length else {
        return Err("Missing Content-Length header".to_string());
    };
    if content_length > MAX_LSP_BODY_BYTES {
        return Err("LSP body exceeds the configured limit".to_string());
    }
    let mut body = vec![0u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("Failed to read LSP body: {error}"))?;
    serde_json::from_slice(&body).map_err(|error| format!("Invalid LSP JSON body: {error}"))
}

fn read_header_line<R: BufRead>(reader: &mut R) -> Result<Option<Vec<u8>>, String> {
    let mut line = Vec::with_capacity(128);
    loop {
        let (bytes_to_consume, line_complete) = {
            let available = reader
                .fill_buf()
                .map_err(|error| format!("Failed to read LSP header: {error}"))?;
            if available.is_empty() {
                if line.is_empty() {
                    return Ok(None);
                }
                return Err("LSP header ended before a line terminator".to_string());
            }
            let remaining = MAX_LSP_HEADER_LINE_BYTES.saturating_sub(line.len());
            if remaining == 0 {
                return Err("LSP header line exceeds the configured limit".to_string());
            }
            let available_len = available.len().min(remaining);
            let newline_index = available[..available_len]
                .iter()
                .position(|byte| *byte == b'\n');
            let bytes_to_consume = newline_index.map_or(available_len, |index| index + 1);
            line.extend_from_slice(&available[..bytes_to_consume]);
            (bytes_to_consume, newline_index.is_some())
        };
        reader.consume(bytes_to_consume);
        if line_complete {
            return Ok(Some(line));
        }
        if line.len() == MAX_LSP_HEADER_LINE_BYTES {
            return Err("LSP header line exceeds the configured limit".to_string());
        }
    }
}
