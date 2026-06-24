//! SSE (Server-Sent Events) parser — dùng cho OpenAI/OpenRouter/Anthropic streaming.
//!
//! Parser xử lý từng chunk byte, accumulate cho đến khi gặp `\n\n` (event boundary).

use std::collections::VecDeque;

/// 1 SSE event: nhiều `field: value` lines, kết thúc bởi empty line.
#[derive(Debug, Clone, Default)]
pub struct SseEvent {
    pub data: String,
    pub event: Option<String>,
    pub id: Option<String>,
}

/// SSE stream parser — accumulate bytes, emit events.
#[derive(Debug, Default)]
pub struct SseParser {
    buffer: String,
    pending: VecDeque<SseEvent>,
}

impl SseParser {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed bytes vào parser. Trả về events hoàn chỉnh.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        let s = std::str::from_utf8(bytes).unwrap_or("");
        self.buffer.push_str(s);

        let mut events = Vec::new();
        while let Some(idx) = self.buffer.find("\n\n") {
            let block: String = self.buffer.drain(..idx + 2).collect();
            let event = parse_block(&block);
            if let Some(ev) = event {
                events.push(ev);
            }
        }
        events
    }

    /// Flush partial buffer (gọi khi stream kết thúc).
    pub fn flush(&mut self) -> Vec<SseEvent> {
        if self.buffer.is_empty() {
            return Vec::new();
        }
        let block = std::mem::take(&mut self.buffer);
        parse_block(&block).into_iter().collect()
    }
}

fn parse_block(block: &str) -> Option<SseEvent> {
    let mut event = SseEvent::default();
    let mut has_data = false;

    for line in block.lines() {
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };

        match field {
            "data" => {
                if has_data {
                    event.data.push('\n');
                }
                event.data.push_str(value);
                has_data = true;
            }
            "event" => event.event = Some(value.to_string()),
            "id" => event.id = Some(value.to_string()),
            _ => {}
        }
    }

    if has_data {
        Some(event)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_event() {
        let mut p = SseParser::new();
        let events = p.feed(b"data: hello world\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello world");
    }

    #[test]
    fn parses_multi_data() {
        let mut p = SseParser::new();
        let events = p.feed(b"data: line1\ndata: line2\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn accumulates_partial() {
        let mut p = SseParser::new();
        let events = p.feed(b"data: hel");
        assert!(events.is_empty());
        let events = p.feed(b"lo\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn ignores_comments() {
        let mut p = SseParser::new();
        let events = p.feed(b": this is a comment\ndata: ok\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "ok");
    }

    #[test]
    fn handles_event_field() {
        let mut p = SseParser::new();
        let events = p.feed(b"event: ping\ndata: 1\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.as_deref(), Some("ping"));
    }
}
