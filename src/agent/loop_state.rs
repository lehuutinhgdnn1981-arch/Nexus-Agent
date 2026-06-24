//! Agent loop state — track iterations, tool calls, cancellation.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

/// Mutable state trong 1 agent turn.
#[derive(Debug)]
pub struct LoopState {
    pub iteration: u32,
    pub tool_calls_made: u32,
    pub max_iterations: u32,
    pub max_tool_calls: u32,
    pub cancel_token: CancellationToken,
}

impl LoopState {
    #[must_use]
    pub fn new(max_iterations: u32, max_tool_calls: u32) -> Self {
        Self {
            iteration: 0,
            tool_calls_made: 0,
            max_iterations,
            max_tool_calls,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Bump iteration count. Trả về false nếu vượt max.
    pub fn bump_iteration(&mut self) -> bool {
        self.iteration += 1;
        self.iteration <= self.max_iterations
    }

    /// Bump tool call count. Trả về false nếu vượt max.
    pub fn bump_tool_call(&mut self) -> bool {
        self.tool_calls_made += 1;
        self.tool_calls_made <= self.max_tool_calls
    }

    /// Kiểm tra cancel.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// Cancel.
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Trả về cancellation token dưới dạng shared handle.
    #[must_use]
    pub fn cancel_handle(&self) -> Arc<CancellationToken> {
        Arc::new(self.cancel_token.clone())
    }
}
