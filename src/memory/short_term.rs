//! Short-term memory — ring buffer per session.

use std::collections::VecDeque;

use crate::llm::types::ChatMessage;

const DEFAULT_CAPACITY: usize = 50;

/// Ring buffer chứa N message gần nhất của 1 session.
#[derive(Debug, Clone)]
pub struct ShortTermMemory {
    buffer: VecDeque<ChatMessage>,
    capacity: usize,
}

impl ShortTermMemory {
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity: capacity.max(1),
        }
    }

    /// Push message. Nếu vượt capacity, loại bỏ message cũ nhất.
    pub fn push(&mut self, msg: ChatMessage) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(msg);
    }

    /// Lấy tất cả message theo thứ tự thời gian.
    #[must_use]
    pub fn all(&self) -> Vec<ChatMessage> {
        self.buffer.iter().cloned().collect()
    }

    /// Lấy N message gần nhất.
    #[must_use]
    pub fn recent(&self, n: usize) -> Vec<ChatMessage> {
        let len = self.buffer.len();
        if n >= len {
            return self.all();
        }
        self.buffer.iter().skip(len - n).cloned().collect()
    }

    /// Clear buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Số message hiện tại.
    #[must_use]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Buffer rỗng?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl Default for ShortTermMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::MessageRole;

    #[test]
    fn ring_buffer_evicts_oldest() {
        let mut m = ShortTermMemory::with_capacity(3);
        m.push(ChatMessage::user("a"));
        m.push(ChatMessage::user("b"));
        m.push(ChatMessage::user("c"));
        m.push(ChatMessage::user("d"));
        assert_eq!(m.len(), 3);
        let all = m.all();
        assert_eq!(all[0].content, "b");
        assert_eq!(all[2].content, "d");
    }

    #[test]
    fn recent_returns_last_n() {
        let mut m = ShortTermMemory::new();
        for i in 0..10 {
            m.push(ChatMessage::user(format!("msg{i}")));
        }
        let recent = m.recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[2].content, "msg9");
    }
}
