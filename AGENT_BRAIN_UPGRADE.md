# NEXUS — Đánh giá "bộ não Agent" + Đề xuất nâng cấp

> Phân tích điểm yếu của agent loop hiện tại, đối chiếu với các framework GitHub nổi, đề xuất cải thiện cụ thể.

---

## 1. Đánh giá bộ não Agent hiện tại

### 1.1 Agent loop hiện tại (ReAct basic)

```rust
loop (max 10 iterations):
    1. LLM chat_stream → emit Delta + ToolCall
    2. For each tool call:
       a. Resolve permission
       b. If !Safe → ApprovalGate (block)
       c. Execute → emit ToolResult
    3. Append results to messages
    4. If no tool calls → Done
```

### 1.2 Điểm mạnh ✅

- **ReAct pattern đúng chuẩn** — Reasoning → Action → Observation → repeat.
- **Streaming + cancellation** — token-by-token, cancel được giữa chừng.
- **Approval gate** — phân quyền rõ ràng, không tự ý thực thi tool nguy hiểm.
- **Sandbox enforcement** — path traversal + symlink escape blocked.
- **Tool auto-discovery** — registry → schema → LLM → tool_call → execute.
- **Max iterations / max tool calls** — chống infinite loop.
- **Memory recall ở đầu turn** — embeddings cosine similarity.
- **DB persistence** — messages, tool_calls, tool_results vào SQLite.

### 1.3 Điểm yếu ❌

#### A. Thiếu planner — không có "thinking step" tách biệt

Hiện tại LLM vừa reasoning vừa chọn tool trong 1 response. Các agent mạnh (Claude 3.5 Sonnet, GPT-4o, DeepSeek-R1) có **reasoning tokens** riêng (chain-of-thought) tách khỏi action. NEXUS không tận dụng được — `assistant_text` chỉ là prose cuối cùng.

#### B. Memory recall thụ động — không có "memory consolidation"

Memory chỉ được recall ở đầu turn với `cosine_similarity(query, memories)`. Không có:
- **Summarization loop** — định kỳ summarize conversation → memory facts.
- **Decay** — memories cũ tự giảm weight theo thời gian.
- **Hierarchical memory** — không phân biệt important vs ephemeral.
- **Working memory** — không giữ "task in progress" riêng.

#### C. Không có reflection / self-critique

Agent không tự đánh giá kết quả tool call trước khi iterate tiếp. Nếu tool fail, agent chỉ thấy error message rồi decide tiếp — không có bước "phân tích nguyên nhân thất bại + điều chỉnh kế hoạch".

#### D. Tool selection kém khi nhiều tools

Khi registry có 24+ tools, LLM phải chọn từ JSON Schema dài → dễ chọn sai tool hoặc hallucinate args. Thiếu:
- **Tool descriptions cải tiến** — ví dụ thêm examples.
- **Dynamic tool subset** — chỉ expose tools liên quan đến task (RAG-style).
- **Tool aliases** — cùng chức năng nhưng tên khác.

#### E. Không có multi-step planning

Hiện tại là 1-shot reactive: user msg → action → result → action → ... → final. Không có:
- **Plan-and-execute** — sinh full plan trước, rồi execute từng step.
- **Plan revision** — adjust plan khi gặp unexpected result.
- **Task decomposition** — chia task lớn thành subtasks.

#### F. Context window quản lý kém

Khi conversation dài, mọi messages được đẩy vào LLM context. Không có:
- **Sliding window** — chỉ giữ N messages gần nhất + summary.
- **Selective compression** — compress old messages thành bullet points.
- **Token counting** — không track token usage per turn.

#### G. Không multi-agent

1 agent xử lý mọi task. Không có:
- **Specialized sub-agents** — coder agent, researcher agent, planner agent.
- **Hand-off** — transfer task giữa agents (OpenAI Swarm pattern).
- **Parallel agents** — chạy song song cho independent tasks.

#### H. Không học từ feedback

Agent không nhớ "lần trước tool X fail với input Y" — lặp lại sai lầm. Thiếu:
- **Episode memory** — log (action → outcome) để tránh lặp.
- **Tool success rate** — track tool reliability, prefer tools có success rate cao.

---

## 2. GitHub repos nổi về Agent "bộ não"

### 2.1 Rust-native (tích hợp trực tiếp được vào NEXUS)

| Repo | Stars | Mô tả | Có thể tích hợp |
|---|---|---|---|
| **[0xPlaygrounds/rig](https://github.com/0xPlaygrounds/rig)** | ~3.4k | Rust LLM framework — vector store, agents, tools, RAG. Design sạch, trait-based. | ⭐⭐⭐⭐⭐ — Thay `LLMProvider` trait bằng rig's `CompletionModel` để có RAG/builders miễn phí. |
| **[Abraxas-365/langchain-rust](https://github.com/Abraxas-365/langchain-rust)** | ~1.1k | Port langchain Python — chains, agents, memory, vector stores. | ⭐⭐⭐⭐ — Lấy ideas về chain composition + memory types. |
| **[bosun-ai/swiftide](https://github.com/bosun-ai/swiftide)** | ~750 | Streaming-first RAG framework. Indexing pipelines + query pipelines. | ⭐⭐⭐⭐ — Lấy `indexing_pipeline` cho memory consolidation. |
| **[DioxusLabs/kalosm](https://github.com/DioxusLabs/kalosm)** | ~1k | Local-first AI framework — model management, agents, RAG. | ⭐⭐⭐ — Thiết kế tốt cho local models (Ollama integration). |
| **[sdiehl/llm-chain](https://github.com/sdiehl/llm-chain)** | ~700 | Mature chains/agents framework. | ⭐⭐⭐ — Ý tưởng chains + prompt templates. |

### 2.2 Python (tham khảo concepts, không integrate)

| Repo | Stars | Mô tả | Lesson cho NEXUS |
|---|---|---|---|
| **[huggingface/smolagents](https://github.com/huggingface/smolagents)** | ~9k | Minimalist "code agent" — LLM sinh Python code thay vì tool calls. | ⭐⭐⭐⭐⭐ — Concept "code agent" rất mạnh: thay vì 24 tools cố định, LLM sinh Python code để gọi tools linh hoạt. |
| **[langchain-ai/langgraph](https://github.com/langchain-ai/langgraph)** | ~10k | State machines cho agents — nodes, edges, conditional routing. | ⭐⭐⭐⭐⭐ — Pattern "state graph" thay cho while loop. Mỗi node = 1 reasoning step, edges = transitions. |
| **[pydantic/pydantic-ai](https://github.com/pydantic/pydantic-ai)** | ~6k | Type-safe agents với Pydantic validation. | ⭐⭐⭐⭐ — Strict validation tool args (NEXUS hiện accept any JSON). |
| **[letta-ai/letta](https://github.com/letta-ai/letta)** (MemGPT) | ~13k | Tiered memory: working memory + archival memory + recall memory. | ⭐⭐⭐⭐⭐ — Pattern memory tiering tuyệt vời. NEXUS có short/long-term nhưng thiếu "working memory" cho task đang chạy. |
| **[openai/swarm](https://github.com/openai/swarm)** | ~18k | Multi-agent handoff pattern — minimalist. | ⭐⭐⭐⭐ — Concept "routines" + handoff giữa agents. |
| **[crewAIInc/crewAI](https://github.com/crewAIInc/crewAI)** | ~25k | Role-based multi-agent (crew của agents với roles/goals). | ⭐⭐⭐ — Pattern "role + goal + backstory" trong system prompt. |
| **[microsoft/autogen](https://github.com/microsoft/autogen)** | ~35k | Conversational multi-agent framework. | ⭐⭐⭐ — Pattern "group chat" giữa agents. |
| **[run-llama/llama_index](https://github.com/run-llama/llama_index)** | ~37k | RAG-first framework — query engines, agents, workflows. | ⭐⭐⭐⭐ — Lấy `Workflow` event-driven pattern. |
| **[Significant-Gravitas/AutoGPT](https://github.com/Significant-Gravitas/AutoGPT)** | ~170k | Autonomous agent với planning + execution + critique. | ⭐⭐⭐ — Pattern plan-execute-critique loop. |

### 2.3 Concept frameworks (academic / experimental)

| Repo | Mô tả |
|---|---|
| **[reflexion-agent/reflexion](https://github.com/reflexion-agent/reflexion)** | Self-reflection pattern — agent tự critique sau mỗi attempt. |
| **[joaomdmoura/crewAI](https://github.com/joaomdmoura/crewAI)** | Role-based agents (đã mention). |
| **[geekan/MetaGPT](https://github.com/geekan/MetaGPT)** | Multi-agent với SOP (Standard Operating Procedure) — mỗi agent có role rõ. |
| **[OpenInterpreter/open-interpreter](https://github.com/OpenInterpreter/open-interpreter)** | Local code interpreter pattern — LLM sinh code chạy local. |

---

## 3. Đề xuất nâng cấp bộ não NEXUS (Priority Order)

### Priority 1: Tích hợp `rig` framework ⭐⭐⭐⭐⭐

```toml
# Cargo.toml
[dependencies]
rig = { version = "0.7", features = ["all"] }
```

**Lợi ích:**
- `rig::agent::AgentBuilder` — thay `Agent` struct của NEXUS, có sẵn RAG, tool calling, streaming.
- `rig::providers::*` — thay 4 provider impls, có sẵn 20+ providers (Together, Groq, Mistral, Hugging Face, ...).
- `rig::vector_store::*` — thay brute-force cosine similarity bằng vector store thật (Qdrant, LanceDB, surrealdb).
- `rig::embeddings::*` — abstraction sạch cho embeddings.

**Trade-off:** Phải refactor `LLMProvider` trait để match rig's `CompletionModel`. Nhưng đáng — Rig là Rust LLM framework tốt nhất hiện tại.

### Priority 2: Memory tiering theo MemGPT pattern ⭐⭐⭐⭐⭐

Thêm 3 tiers:

```rust
pub enum MemoryTier {
    /// Trong context window của LLM — messages gần đây.
    Working,
    /// Facts/preferences recall được qua embedding search.
    Archival,
    /// Summary của conversations cũ — nén để tiết kiệm context.
    Recall,
}
```

**Implementation:**
- Background task: định kỳ (mỗi 50 messages) summarize conversation → tạo `Recall` memory.
- Context builder: `WorkingMemory + relevant Archival + relevant Recall` → system message.
- Decay: Archival memories có `use_count < 5` và `last_used_at > 30 days` → move to `Recall`.

### Priority 3: Plan-and-execute pattern ⭐⭐⭐⭐

```rust
pub struct AgentPlan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub current_step: usize,
}

pub struct PlanStep {
    pub description: String,
    pub expected_tools: Vec<String>,
    pub status: StepStatus, // Pending | InProgress | Done | Failed | Skipped
    pub result: Option<String>,
}
```

**Flow:**
1. User msg → LLM sinh plan (1 turn).
2. Execute step-by-step (mỗi step là 1 mini-ReAct loop).
3. Sau mỗi step, LLM có thể revise plan.
4. Sau tất cả steps → final summary.

**Khi nào dùng:** Heuristic — nếu user msg chứa "and", "then", "after that" hoặc dài >100 chars → bật plan-execute.

### Priority 4: Code agent pattern (smolagents) ⭐⭐⭐⭐

Thêm tool `run_python` đã có, nhưng nâng cấp thành **code agent mode**:
- Thay vì 24 tools cố định, LLM sinh Python code sử dụng `nexus` SDK:

```python
from nexus import files, shell, browser, search

# LLM-generated
result = files.read("data.csv")
processed = process_csv(result)  # arbitrary logic
shell.run(f"echo '{processed}' > output.txt")
```

**Lợi ích:**
- Composability — LLM viết loops, conditionals, transformations.
- Fewer tools — `nexus` SDK có 6 modules, mỗi module vài functions.
- Debug-friendly — code chạy trong workspace, có stdout/stderr.

### Priority 5: Reflection loop ⭐⭐⭐

Sau mỗi iteration, nếu tool call fail:

```rust
async fn reflect_on_failure(
    &self,
    tool: &str,
    input: &Value,
    error: &str,
    history: &[ChatMessage],
) -> Option<ReflectionResult> {
    // LLM phân tích: "Tại sao tool X fail với input Y?"
    // Trả về: { root_cause, alternative_approach, retry_input }
}
```

### Priority 6: Multi-agent (Swarm pattern) ⭐⭐⭐

```rust
pub struct SubAgent {
    pub name: String,           // "coder", "researcher", "planner"
    pub instructions: String,   // role-specific system prompt
    pub tools: Vec<String>,     // subset of tools
}

// Handoff: agent A decide "this needs coding" → transfer to coder agent
pub fn handoff(from: &str, to: &str, context: &Context) -> Result<()>;
```

### Priority 7: LangGraph-style state machine ⭐⭐⭐

```rust
pub enum AgentState {
    Planning,
    Executing { plan: AgentPlan, step_idx: usize },
    Reflecting { failed_step: usize, error: String },
    Done { result: String },
}

// Edges: Planning → Executing → (Done | Reflecting → Executing)
```

### Priority 8: Token counting + context compression ⭐⭐

```rust
pub struct ContextManager {
    max_tokens: usize,
    current_tokens: usize,
    messages: Vec<ChatMessage>,
}

impl ContextManager {
    pub fn add_message(&mut self, msg: ChatMessage);
    pub fn compress_if_needed(&mut self); // summarize old messages
    pub fn build_context(&self) -> Vec<ChatMessage>;
}
```

---

## 4. Roadmap thực thi (3-phase upgrade)

### Phase A (1-2 weeks): Memory + Context
1. ✅ Custom providers (đã làm ở trên)
2. Memory tiering (Working / Archival / Recall)
3. Background summarization loop
4. Token counting + context compression
5. Replace brute-force cosine với LanceDB local vector store

### Phase B (2-4 weeks): Planning + Reflection
1. Plan-and-execute mode (heuristic trigger)
2. Reflection loop cho failed tool calls
3. Episode memory (log success/fail patterns)
4. Dynamic tool subset selection (RAG-style: query → relevant tools only)

### Phase C (4-8 weeks): Multi-agent + Code agent
1. Sub-agent abstraction (coder/researcher/planner)
2. Handoff pattern (OpenAI Swarm style)
3. Code agent mode (smolagents pattern với Python SDK)
4. Optional: tích hợp `rig` framework thay LLMProvider trait

---

## 5. Kết luận

Bộ não Agent hiện tại của NEXUS là **ReAct basic tốt** — đủ để ship v0.1.0. Nhưng để cạnh tranh với các agent mạnh (Claude Code, Cursor, Continue, Aider), cần upgrade:

1. **Ngắn hạn (Phase A)**: memory tiering + context compression — giảm hallucination, tăng long-conversation capability.
2. **Trung hạn (Phase B)**: plan-and-execute + reflection — giải quyết tasks phức tạp hơn.
3. **Dài hạn (Phase C)**: multi-agent + code agent — composability và flexibility.

**Recommend**: tích hợp **`rig` framework** làm base layer. Rig đã giải quyết rất nhiều bài toán (providers, vector store, embeddings, agents) mà NEXUS đang tự implement thủ công. Thay vì reinvent, compose.

**Repo đáng học nhất**: 
- **smolagents** — cho code agent pattern.
- **langgraph** — cho state machine agent loop.
- **letta (MemGPT)** — cho memory tiering.
- **rig** — cho Rust implementation reference.
