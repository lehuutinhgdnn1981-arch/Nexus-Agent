//! LLM streaming integration tests with recorded fixtures.
//!
//! Tests the SSE parser against recorded OpenAI/Anthropic/Ollama responses.

#![cfg(feature = "test-utils")]

use nexus::llm::streaming::SseParser;

// === OpenAI SSE fixtures ===

const OPENAI_SSE_FIXTURE: &str = r#"data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"}}]}

data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{"content":" world"}}]}

data: {"id":"chatcmpl-1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7}}

data: [DONE]

"#;

const OPENAI_TOOL_CALL_FIXTURE: &str = r#"data: {"id":"chatcmpl-2","choices":[{"index":0,"delta":{"role":"assistant","content":null,"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"read_file","arguments":""}}]}}]}

data: {"id":"chatcmpl-2","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":"}}]}}]}

data: {"id":"chatcmpl-2","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":" \"test.txt\"}"}}]}}]}

data: {"id":"chatcmpl-2","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}

data: [DONE]

"#;

// === Anthropic SSE fixtures ===

const ANTHROPIC_SSE_FIXTURE: &str = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"claude-3-5-sonnet","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":", world!"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}

event: message_stop
data: {"type":"message_stop"}

"#;

#[test]
fn parses_openai_text_stream() {
    let mut parser = SseParser::new();
    let events = parser.feed(OPENAI_SSE_FIXTURE.as_bytes());

    let data_events: Vec<_> = events.into_iter().filter(|e| e.data != "[DONE]").collect();
    assert_eq!(data_events.len(), 3);

    // First event: content "Hello"
    let v0: serde_json::Value = serde_json::from_str(&data_events[0].data).unwrap();
    assert_eq!(
        v0["choices"][0]["delta"]["content"],
        "Hello"
    );

    // Second event: content " world"
    let v1: serde_json::Value = serde_json::from_str(&data_events[1].data).unwrap();
    assert_eq!(v1["choices"][0]["delta"]["content"], " world");

    // Third event: usage
    let v2: serde_json::Value = serde_json::from_str(&data_events[2].data).unwrap();
    assert_eq!(v2["usage"]["total_tokens"], 7);
}

#[test]
fn parses_openai_tool_call_stream() {
    let mut parser = SseParser::new();
    let events = parser.feed(OPENAI_TOOL_CALL_FIXTURE.as_bytes());

    let data_events: Vec<_> = events.into_iter().filter(|e| e.data != "[DONE]").collect();
    assert_eq!(data_events.len(), 4);

    // First event: tool call name "read_file"
    let v0: serde_json::Value = serde_json::from_str(&data_events[0].data).unwrap();
    assert_eq!(
        v0["choices"][0]["delta"]["tool_calls"][0]["function"]["name"],
        "read_file"
    );

    // Subsequent events: tool call arguments accumulated
    let v1: serde_json::Value = serde_json::from_str(&data_events[1].data).unwrap();
    let args1 = v1["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"]
        .as_str()
        .unwrap();
    assert!(args1.contains("\"path\":"));

    let v2: serde_json::Value = serde_json::from_str(&data_events[2].data).unwrap();
    let args2 = v2["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"]
        .as_str()
        .unwrap();
    assert!(args2.contains("test.txt"));
}

#[test]
fn parses_anthropic_text_stream() {
    let mut parser = SseParser::new();
    let events = parser.feed(ANTHROPIC_SSE_FIXTURE.as_bytes());

    assert!(events.len() >= 7);

    // First event: message_start
    let v0: serde_json::Value = serde_json::from_str(&events[0].data).unwrap();
    assert_eq!(v0["type"], "message_start");

    // content_block_delta events
    let deltas: Vec<_> = events
        .iter()
        .filter_map(|e| {
            let v: serde_json::Value = serde_json::from_str(&e.data).ok()?;
            if v.get("type")?.as_str()? == "content_block_delta" {
                Some(v)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(deltas.len(), 2);
    assert_eq!(deltas[0]["delta"]["text"], "Hello");
    assert_eq!(deltas[1]["delta"]["text"], ", world!");

    // message_stop
    let stop = events.iter().find(|e| {
        let v: serde_json::Value = serde_json::from_str(&e.data).unwrap();
        v.get("type").and_then(|t| t.as_str()) == Some("message_stop")
    });
    assert!(stop.is_some());
}

#[test]
fn handles_partial_chunks() {
    let mut parser = SseParser::new();

    // Feed partial chunk
    let events1 = parser.feed(b"data: {\"a\":1");
    assert!(events1.is_empty());

    // Complete the chunk
    let events2 = parser.feed(b"}\n\n");
    assert_eq!(events2.len(), 1);
    let v: serde_json::Value = serde_json::from_str(&events2[0].data).unwrap();
    assert_eq!(v["a"], 1);
}

#[test]
fn handles_empty_data() {
    let mut parser = SseParser::new();
    let events = parser.feed(b"data: \n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "");
}

#[test]
fn ignores_comments() {
    let mut parser = SseParser::new();
    let events = parser.feed(b": this is a comment\ndata: ok\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "ok");
}

#[test]
fn handles_multiple_events_in_one_feed() {
    let mut parser = SseParser::new();
    let input = b"data: first\n\ndata: second\n\ndata: third\n\n";
    let events = parser.feed(input);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].data, "first");
    assert_eq!(events[1].data, "second");
    assert_eq!(events[2].data, "third");
}

#[test]
fn handles_multiline_data() {
    let mut parser = SseParser::new();
    let events = parser.feed(b"data: line1\ndata: line2\ndata: line3\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "line1\nline2\nline3");
}

#[test]
fn flush_emits_partial() {
    let mut parser = SseParser::new();
    let _ = parser.feed(b"data: hello");
    let events = parser.flush();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "hello");
}

// === Ollama NDJSON fixtures ===

const OLLAMA_NDJSON_FIXTURE: &str = r#"{"model":"llama3.1","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}
{"model":"llama3.1","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":", world!"},"done":false}
{"model":"llama3.1","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":""},"done":true,"prompt_eval_count":10,"eval_count":2}
"#;

#[test]
fn parses_ollama_ndjson_stream() {
    let lines: Vec<&str> = OLLAMA_NDJSON_FIXTURE
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(lines.len(), 3);

    let v0: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v0["message"]["content"], "Hello");
    assert_eq!(v0["done"], false);

    let v1: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(v1["message"]["content"], ", world!");

    let v2: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(v2["done"], true);
    assert_eq!(v2["prompt_eval_count"], 10);
    assert_eq!(v2["eval_count"], 2);
}
