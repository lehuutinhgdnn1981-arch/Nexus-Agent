//! Agent — ReAct loop implementation.
//!
//! Workflow per turn:
//!   1. Build system prompt + tools schema.
//!   2. Push user message into short-term memory + DB.
//!   3. Loop (max 10 iterations):
//!      a. LLM chat_stream → emit Delta events.
//!      b. If tool calls returned → for each: approval check → execute → observe.
//!      c. If no tool calls → done.
//!   4. Persist final assistant message to DB.

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::agent::brain::Brain;
use crate::agent::config::AgentRuntimeConfig;
use crate::agent::event::AgentEvent;
use crate::agent::loop_state::LoopState;
use crate::agent::prompt::build_system_prompt;
use crate::database::repositories::message_repo::MessageRepo;
use crate::error::{NexusError, Result};
use crate::llm::provider::LLMProvider;
use crate::llm::types::{ChatMessage, ChatRequest, ChatStreamChunk};
use crate::security::approval::{ApprovalDecision, ApprovalGate, ApprovalRequest};
use crate::state::AppState;
use crate::llm::types::ToolCall;
use crate::tools::tool::ToolResult;

/// Agent — stateless per turn, but holds references to shared state.
///
/// V2: optional `brain` field enables memory tiering, context compression,
/// plan-and-execute, reflection, dynamic tool subset, episode memory.
pub struct Agent {
    pub state: Arc<AppState>,
    pub config: AgentRuntimeConfig,
    pub provider: Arc<dyn LLMProvider>,
    /// Brain module — None = legacy ReAct (v0 behavior).
    pub brain: Option<Arc<Brain>>,
}

impl Agent {
    pub fn new(
        state: Arc<AppState>,
        config: AgentRuntimeConfig,
        provider: Arc<dyn LLMProvider>,
    ) -> Self {
        Self {
            state,
            config,
            provider,
            brain: None,
        }
    }

    /// Attach Brain module — enables V2 features.
    pub fn with_brain(mut self, brain: Arc<Brain>) -> Self {
        self.brain = Some(brain);
        self
    }

    /// Run 1 agent turn. `tx` nhận events realtime (cho IPC streaming).
    pub async fn run(
        &self,
        session_id: &str,
        user_message: &str,
        tx: mpsc::Sender<AgentEvent>,
    ) -> Result<()> {
        let run_id = crate::utils::ids::new_uuid();
        let _ = tx
            .send(AgentEvent::TurnStart {
                run_id: run_id.clone(),
                session_id: session_id.to_string(),
                user_message: user_message.to_string(),
            })
            .await;

        // Register cancellation token
        let cancel_token = {
            let token = tokio_util::sync::CancellationToken::new();
            self.state.active_runs.insert(run_id.clone(), token.clone());
            token
        };

        let result = self
            .run_inner(&run_id, session_id, user_message, &tx, &cancel_token)
            .await;

        // Cleanup
        self.state.active_runs.remove(&run_id);

        match result {
            Ok(()) => Ok(()),
            Err(NexusError::Cancelled) => {
                let _ = tx.send(AgentEvent::Cancelled { run_id }).await;
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "agent turn failed");
                let _ = tx
                    .send(AgentEvent::Error {
                        run_id,
                        message: e.to_string(),
                    })
                    .await;
                Err(e)
            }
        }
    }

    async fn run_inner(
        &self,
        run_id: &str,
        session_id: &str,
        user_message: &str,
        tx: &mpsc::Sender<AgentEvent>,
        cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        // Persist user message
        let user_msg_id = crate::utils::ids::new_uuid();
        MessageRepo::append(
            &self.state.pool,
            &user_msg_id,
            session_id,
            "user",
            user_message,
            None,
            None,
        )
        .await?;

        // Push to short-term memory
        self.state
            .memory
            .push_short_term(session_id, ChatMessage::user(user_message))
            .await;

        // === V2: Brain-powered context building ===
        let tools = if let Some(brain) = &self.brain {
            // Dynamic tool subset — chỉ expose relevant tools
            brain.select_tools(user_message, &self.state.tool_registry).await
                .unwrap_or_else(|_| self.state.tool_registry.all_schemas())
        } else {
            self.state.tool_registry.all_schemas()
        };

        let system_msg = build_system_prompt(self.config.system_prompt.as_deref(), &tools);

        // Build messages: short-term + tiered recall + compression
        let short_term = self.state.memory.short_term_all(session_id).await;
        let mut messages: Vec<ChatMessage> = if let Some(brain) = &self.brain {
            // V2: use brain.build_context for tiering + compression
            brain.build_context(session_id, &short_term, user_message).await
                .unwrap_or_else(|e| {
                    warn!(error = %e, "brain context build failed, falling back to short-term");
                    short_term
                })
        } else {
            // V0: just short-term + recall (legacy)
            let mut msgs = short_term;
            if !user_message.is_empty() {
                let query = crate::memory::model::MemoryQuery::new(user_message);
                match self.state.memory.recall(&query).await {
                    Ok(recalled) if !recalled.is_empty() => {
                        let mem_text = recalled
                            .iter()
                            .map(|m| format!("- [{}] {}", m.category.as_str(), m.content))
                            .collect::<Vec<_>>()
                            .join("\n");
                        let mem_msg = ChatMessage::system(format!("Relevant memories:\n{mem_text}"));
                        msgs.insert(0, mem_msg);
                    }
                    Ok(_) => {}
                    Err(e) => warn!(error = %e, "memory recall failed"),
                }
            }
            msgs
        };
        // Prepend system message
        messages.insert(0, system_msg);

        // === V2: Episode memory — warn about similar past failures ===
        if let Some(brain) = &self.brain {
            // Check if user message mentions a tool that failed before
            let tool_names: Vec<String> = self.state.tool_registry.list_names();
            for name in &tool_names {
                if user_message.to_lowercase().contains(name) {
                    if let Ok(failures) = brain
                        .find_similar_failures(name, &serde_json::json!({"query": user_message}))
                        .await
                    {
                        if !failures.is_empty() {
                            let warning = failures[0].to_warning();
                            messages.insert(1, ChatMessage::system(warning));
                            break;
                        }
                    }
                }
            }
        }

        let mut loop_state = LoopState::new(self.config.max_iterations, self.config.max_tool_calls);

        // Main loop
        while loop_state.bump_iteration() {
            if cancel_token.is_cancelled() {
                return Err(NexusError::Cancelled);
            }

            // Build request — only include tools if provider supports them
            let req = if self.provider.supports_tools() {
                ChatRequest::new(&self.config.default_model, messages.clone())
                    .with_tools(tools.clone())
            } else {
                // Provider doesn't support tools — send plain chat request
                // Agent will respond with text only, no tool calls
                ChatRequest::new(&self.config.default_model, messages.clone())
            };

            let (stx, mut srx) = mpsc::channel::<ChatStreamChunk>(64);
            let provider = Arc::clone(&self.provider);
            let req_clone = req.clone();
            let stream_handle = tokio::spawn(async move {
                provider.chat_stream(req_clone, stx).await
            });

            let mut assistant_text = String::new();
            let mut tool_calls: Vec<ToolCall> = Vec::new();
            let mut usage = crate::llm::types::Usage::default();

            while let Some(chunk) = srx.recv().await {
                if cancel_token.is_cancelled() {
                    let _ = stream_handle.await;
                    return Err(NexusError::Cancelled);
                }
                match chunk {
                    ChatStreamChunk::Delta(t) => {
                        assistant_text.push_str(&t);
                        let _ = tx
                            .send(AgentEvent::Delta {
                                run_id: run_id.to_string(),
                                session_id: session_id.to_string(),
                                text: t,
                            })
                            .await;
                    }
                    ChatStreamChunk::ToolCall(tc) => {
                        tool_calls.push(tc);
                    }
                    ChatStreamChunk::Usage(u) => {
                        usage = u;
                    }
                    ChatStreamChunk::Done => break,
                }
            }

            // Check stream errors
            match stream_handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    // If error mentions "tool" — retry without tools
                    let err_str = e.to_string();
                    if err_str.contains("tool") || err_str.contains("404") {
                        warn!(error = %e, "LLM rejected tools — retrying without tools");
                        let req_no_tools = ChatRequest::new(&self.config.default_model, messages.clone());
                        let (stx2, mut srx2) = mpsc::channel::<ChatStreamChunk>(64);
                        let provider2 = Arc::clone(&self.provider);
                        let req2 = req_no_tools.clone();
                        let handle2 = tokio::spawn(async move {
                            provider2.chat_stream(req2, stx2).await
                        });

                        while let Some(chunk) = srx2.recv().await {
                            if cancel_token.is_cancelled() {
                                let _ = handle2.await;
                                return Err(NexusError::Cancelled);
                            }
                            match chunk {
                                ChatStreamChunk::Delta(t) => {
                                    assistant_text.push_str(&t);
                                    let _ = tx.send(AgentEvent::Delta {
                                        run_id: run_id.to_string(),
                                        session_id: session_id.to_string(),
                                        text: t,
                                    }).await;
                                }
                                ChatStreamChunk::Done => break,
                                _ => {}
                            }
                        }
                        match handle2.await {
                            Ok(Ok(())) => {}
                            Ok(Err(e2)) => return Err(e2.into()),
                            Err(e2) => return Err(NexusError::Internal(format!("stream task: {e2}"))),
                        }
                    } else {
                        return Err(e.into());
                    }
                }
                Err(e) => return Err(NexusError::Internal(format!("stream task: {e}"))),
            }

            // Build assistant message + persist
            let assistant_msg = if tool_calls.is_empty() {
                ChatMessage::assistant(&assistant_text)
            } else {
                ChatMessage::assistant_with_tools(&assistant_text, tool_calls.clone())
            };
            messages.push(assistant_msg.clone());

            let tool_calls_json = if tool_calls.is_empty() {
                None
            } else {
                Some(serde_json::to_value(&tool_calls)?)
            };

            let assistant_msg_id = crate::utils::ids::new_uuid();
            MessageRepo::append(
                &self.state.pool,
                &assistant_msg_id,
                session_id,
                "assistant",
                &assistant_text,
                tool_calls_json.as_ref(),
                None,
            )
            .await?;

            self.state
                .memory
                .push_short_term(session_id, assistant_msg)
                .await;

            // No tool calls → done
            if tool_calls.is_empty() {
                let _ = tx
                    .send(AgentEvent::Done {
                        run_id: run_id.to_string(),
                        session_id: session_id.to_string(),
                        final_message: assistant_text.clone(),
                        usage,
                    })
                    .await;
                return Ok(());
            }

            // Execute tool calls
            for tc in &tool_calls {
                if cancel_token.is_cancelled() {
                    return Err(NexusError::Cancelled);
                }
                if !loop_state.bump_tool_call() {
                    warn!("max_tool_calls reached, stopping");
                    break;
                }

                let result = self
                    .execute_tool_call(run_id, session_id, tc, tx, cancel_token)
                    .await?;

                // Push tool result message
                let tool_msg = ChatMessage::tool_result(&tc.id, &tc.function.name, &result.output);
                messages.push(tool_msg.clone());

                // Persist
                let tool_msg_id = crate::utils::ids::new_uuid();
                let tr_json = serde_json::to_value(&result)?;
                MessageRepo::append(
                    &self.state.pool,
                    &tool_msg_id,
                    session_id,
                    "tool",
                    &result.output,
                    None,
                    Some(&tr_json),
                )
                .await?;

                self.state
                    .memory
                    .push_short_term(session_id, tool_msg)
                    .await;
            }

            let _ = tx
                .send(AgentEvent::IterationEnd {
                    run_id: run_id.to_string(),
                    iteration: loop_state.iteration,
                    tool_calls_made: loop_state.tool_calls_made,
                })
                .await;
        }

        // Hit max iterations
        warn!(iterations = loop_state.iteration, "max_iterations reached");
        let _ = tx
            .send(AgentEvent::Done {
                run_id: run_id.to_string(),
                session_id: session_id.to_string(),
                final_message: "[agent reached max iterations]".to_string(),
                usage: crate::llm::types::Usage::default(),
            })
            .await;
        Ok(())
    }

    /// Execute 1 tool call — check permission, request approval if needed, run.
    async fn execute_tool_call(
        &self,
        run_id: &str,
        session_id: &str,
        tc: &ToolCall,
        tx: &mpsc::Sender<AgentEvent>,
        cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<ToolResult> {
        let tool = match self.state.tool_registry.get(&tc.function.name) {
            Some(t) => t,
            None => {
                return Ok(ToolResult::error(
                    &tc.id,
                    &tc.function.name,
                    format!("tool `{}` not found in registry", tc.function.name),
                ));
            }
        };

        let permission = tool.permission();
        let input: serde_json::Value = serde_json::from_str(&tc.function.arguments)
            .unwrap_or(serde_json::Value::Null);

        // Approval flow
        if permission.requires_approval() {
            if cancel_token.is_cancelled() {
                return Err(NexusError::Cancelled);
            }
            let request_id = ApprovalGate::new_id();
            let req = ApprovalRequest {
                id: request_id.clone(),
                tool: tc.function.name.clone(),
                input: input.clone(),
                permission,
                session_id: Some(session_id.to_string()),
                run_id: run_id.to_string(),
            };
            let _ = tx
                .send(AgentEvent::ApprovalRequest {
                    run_id: run_id.to_string(),
                    request_id: request_id.clone(),
                    tool: tc.function.name.clone(),
                    input: input.clone(),
                    permission: permission.label().to_string(),
                })
                .await;

            let app_state = Arc::clone(&self.state);
            let req_clone = req.clone();
            let decision = self
                .state
                .approval_gate
                .request(req, move |r| {
                    // Emit IPC event directly (caller wires this up via state)
                    let _ = r;
                    let _ = &app_state;
                })
                .await?;

            if decision == ApprovalDecision::Rejected {
                return Ok(ToolResult::error(
                    &tc.id,
                    &tc.function.name,
                    "user rejected this tool call".to_string(),
                ));
            }
            let _ = req_clone;
        }

        // Emit ToolCallStart
        let _ = tx
            .send(AgentEvent::ToolCallStart {
                run_id: run_id.to_string(),
                call_id: tc.id.clone(),
                tool: tc.function.name.clone(),
                input: input.clone(),
            })
            .await;

        // Build ToolContext
        let ctx = crate::tools::context::ToolContext {
            session_id: Some(session_id.to_string()),
            run_id: Some(run_id.to_string()),
            workspace: Arc::clone(&self.state.sandbox),
            pool: self.state.pool.clone(),
            memory: Arc::clone(&self.state.memory),
            browser: Arc::clone(&self.state.browser),
            scheduler: Arc::clone(&self.state.scheduler),
            config: Arc::clone(&self.state.config),
        };

        // Execute
        let started = std::time::Instant::now();
        let result = tool.execute(&ctx, input.clone()).await;
        let elapsed = started.elapsed();

        let tool_result = match result {
            Ok(r) => r,
            Err(e) => {
                warn!(tool = %tc.function.name, error = %e, "tool execution failed");
                ToolResult::error(&tc.id, &tc.function.name, e.to_string())
            }
        };

        debug!(tool = %tc.function.name, elapsed_ms = elapsed.as_millis() as u64, "tool executed");

        // === V2: Record episode (success or failure) ===
        if let Some(brain) = &self.brain {
            let success = tool_result.ok;
            let error_msg = if success { None } else { Some(tool_result.output.as_str()) };
            if let Err(e) = brain
                .record_episode(
                    &tc.function.name,
                    &input,
                    success,
                    error_msg,
                    Some(session_id),
                )
                .await
            {
                warn!(error = %e, "failed to record episode");
            }
        }

        // === V2: Reflect on failure ===
        if !tool_result.ok {
            if let Some(brain) = &self.brain {
                // Build a small history snapshot from short-term memory for reflection context.
                // (Passing empty Vec is OK — reflection works with just tool + input + error.)
                let history: Vec<ChatMessage> = Vec::new();
                match brain
                    .reflect(
                        &tc.function.name,
                        &input,
                        &tool_result.output,
                        &history,
                    )
                    .await
                {
                    Ok(reflection) if reflection.should_skip => {
                        info!(
                            tool = %tc.function.name,
                            root_cause = %reflection.root_cause,
                            "reflection suggests skip — continuing"
                        );
                    }
                    Ok(reflection) if reflection.should_retry => {
                        if let Some(revised) = reflection.revised_input {
                            info!(
                                tool = %tc.function.name,
                                "reflection suggests retry with revised input"
                            );
                            // Retry with revised input (1 retry max — avoid infinite recursion)
                            let retry_result = tool.execute(&ctx, revised.clone()).await;
                            let retry_result = match retry_result {
                                Ok(r) => r,
                                Err(e) => ToolResult::error(&tc.id, &tc.function.name, e.to_string()),
                            };
                            // Record retry episode
                            if let Err(e) = brain
                                .record_episode(
                                    &tc.function.name,
                                    &revised,
                                    retry_result.ok,
                                    if retry_result.ok { None } else { Some(retry_result.output.as_str()) },
                                    Some(session_id),
                                )
                                .await
                            {
                                warn!(error = %e, "failed to record retry episode");
                            }
                            // Emit ToolCallEnd with retry result
                            let _ = tx
                                .send(AgentEvent::ToolCallEnd {
                                    run_id: run_id.to_string(),
                                    call_id: tc.id.clone(),
                                    result: retry_result.clone(),
                                })
                                .await;
                            return Ok(retry_result);
                        }
                    }
                    Ok(_) => {
                        info!(tool = %tc.function.name, "reflection suggests continue without retry");
                    }
                    Err(e) => {
                        warn!(error = %e, "reflection failed");
                    }
                }
            }
        }

        // Emit ToolCallEnd
        let _ = tx
            .send(AgentEvent::ToolCallEnd {
                run_id: run_id.to_string(),
                call_id: tc.id.clone(),
                result: tool_result.clone(),
            })
            .await;

        Ok(tool_result)
    }
}
