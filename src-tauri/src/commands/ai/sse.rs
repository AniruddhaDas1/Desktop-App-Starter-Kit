use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;

/// Read server-sent events from a streaming response until it ends or the
/// request is cancelled. Events are separated by a blank line (LF or CRLF).
pub async fn for_each_event<F>(
    response: reqwest::Response,
    token: CancellationToken,
    mut on_event: F,
) -> Result<(), AppError>
where
    F: FnMut(&str) -> Result<(), AppError>,
{
    let mut stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();

    loop {
        tokio::select! {
            _ = token.cancelled() => return Ok(()),
            next = stream.next() => {
                let Some(chunk) = next else { break };
                let chunk = chunk.map_err(|err| AppError::new(err.to_string()))?;
                buffer.extend_from_slice(&chunk);

                while let Some(event) = next_event(&mut buffer) {
                    on_event(&String::from_utf8_lossy(&event))?;
                }
            }
        }
    }

    if !buffer.iter().all(u8::is_ascii_whitespace) {
        on_event(&String::from_utf8_lossy(&buffer))?;
    }

    Ok(())
}

fn next_event(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
    let lf = find(buffer, b"\n\n").map(|index| (index, 2));
    let crlf = find(buffer, b"\r\n\r\n").map(|index| (index, 4));

    let (index, length) = match (lf, crlf) {
        (Some(left), Some(right)) => {
            if left.0 <= right.0 {
                left
            } else {
                right
            }
        }
        (Some(found), None) | (None, Some(found)) => found,
        (None, None) => return None,
    };

    let event = buffer[..index].to_vec();
    buffer.drain(..index + length);
    Some(event)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_split_on_lf_and_crlf() {
        let mut buffer = b"data: one\n\ndata: two\r\n\r\ndata: thr".to_vec();

        assert_eq!(next_event(&mut buffer).unwrap(), b"data: one");
        assert_eq!(next_event(&mut buffer).unwrap(), b"data: two");
        assert!(next_event(&mut buffer).is_none());
        assert_eq!(buffer, b"data: thr");
    }

    #[test]
    fn multibyte_text_is_not_corrupted_at_event_boundaries() {
        let mut buffer = "data: héllo\n\n".as_bytes().to_vec();
        let event = next_event(&mut buffer).unwrap();
        assert_eq!(String::from_utf8(event).unwrap(), "data: héllo");
    }
}
