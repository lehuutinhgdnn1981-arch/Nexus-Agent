//! File tools — unit tests.

#![cfg(test)]

use std::sync::Arc;

use serde_json::json;

use super::super::context::ToolContext;
use crate::browser::BrowserManager;
use crate::config::AppConfig;
use crate::database::pool::in_memory_pool;
use crate::llm::factory::build_provider;
use crate::memory::embedding::EmbeddingClient;
use crate::memory::store::MemoryStore;
use crate::scheduler::SchedulerService;
use crate::security::Sandbox;
use crate::tools::file::{
    append_file::AppendFileTool, copy_file::CopyFileTool,
    create_directory::CreateDirectoryTool, delete_file::DeleteFileTool,
    list_directory::ListDirectoryTool, move_file::MoveFileTool, read_file::ReadFileTool,
    search_files::SearchFilesTool, write_file::WriteFileTool,
};
use crate::tools::tool::Tool;

async fn make_ctx(workspace_root: std::path::PathBuf) -> ToolContext {
    let pool = in_memory_pool().await.unwrap();
    let config = Arc::new(AppConfig::defaults());
    let sandbox = Arc::new(Sandbox::new(workspace_root));
    let provider = build_provider(
        "openai",
        &config.llm.openai,
        &config.memory.embedding_model,
    )
    .unwrap_or_else(|_| {
        // Fallback: dummy provider — không dùng embedding trong file tests
        build_provider("ollama", &config.llm.ollama, &config.memory.embedding_model).unwrap()
    });
    let embedding_client = Arc::new(EmbeddingClient::new(provider));
    let memory = Arc::new(MemoryStore::new(pool.clone(), embedding_client, 0.92));
    let browser = Arc::new(BrowserManager::new(true, 9222));
    let on_fire: crate::scheduler::service::FireCallback = Arc::new(|_, _, _| {});
    let scheduler = Arc::new(SchedulerService::new(pool.clone(), on_fire));

    ToolContext {
        session_id: Some("test_session".into()),
        run_id: Some("test_run".into()),
        workspace: sandbox,
        pool,
        memory,
        browser,
        scheduler,
        config,
    }
}

#[tokio::test]
async fn write_and_read_file() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let write = WriteFileTool;
    let result = write
        .execute(&ctx, json!({"path": "hello.txt", "content": "Hello, NEXUS!"}))
        .await
        .unwrap();
    assert!(result.ok);
    assert!(result.output.contains("wrote"));

    let read = ReadFileTool;
    let result = read
        .execute(&ctx, json!({"path": "hello.txt"}))
        .await
        .unwrap();
    assert!(result.ok);
    assert_eq!(result.output, "Hello, NEXUS!");
}

#[tokio::test]
async fn append_file() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let write = WriteFileTool;
    write
        .execute(&ctx, json!({"path": "log.txt", "content": "line1\n"}))
        .await
        .unwrap();

    let append = AppendFileTool;
    append
        .execute(&ctx, json!({"path": "log.txt", "content": "line2\n"}))
        .await
        .unwrap();

    let read = ReadFileTool;
    let result = read
        .execute(&ctx, json!({"path": "log.txt"}))
        .await
        .unwrap();
    assert_eq!(result.output, "line1\nline2\n");
}

#[tokio::test]
async fn move_and_copy_file() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let write = WriteFileTool;
    write
        .execute(&ctx, json!({"path": "src.txt", "content": "data"}))
        .await
        .unwrap();

    let copy = CopyFileTool;
    let r = copy
        .execute(&ctx, json!({"src": "src.txt", "dst": "dst.txt"}))
        .await
        .unwrap();
    assert!(r.ok);

    let mv = MoveFileTool;
    let r = mv
        .execute(&ctx, json!({"src": "src.txt", "dst": "moved.txt"}))
        .await
        .unwrap();
    assert!(r.ok);

    // src.txt gone, dst.txt + moved.txt exist
    let list = ListDirectoryTool;
    let r = list.execute(&ctx, json!({})).await.unwrap();
    let data = r.data.unwrap();
    let names: Vec<String> = data["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"dst.txt".to_string()));
    assert!(names.contains(&"moved.txt".to_string()));
    assert!(!names.contains(&"src.txt".to_string()));
}

#[tokio::test]
async fn list_directory_sorted() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let write = WriteFileTool;
    for name in &["b.txt", "a.txt", "c.txt"] {
        write
            .execute(&ctx, json!({"path": name, "content": "x"}))
            .await
            .unwrap();
    }

    let list = ListDirectoryTool;
    let r = list.execute(&ctx, json!({})).await.unwrap();
    let names: Vec<String> = r.data.unwrap()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["a.txt", "b.txt", "c.txt"]);
}

#[tokio::test]
async fn create_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let tool = CreateDirectoryTool;
    let r = tool
        .execute(&ctx, json!({"path": "nested/deep/dir"}))
        .await
        .unwrap();
    assert!(r.ok);

    let list = ListDirectoryTool;
    let _ = list
        .execute(&ctx, json!({"path": "nested/deep/dir"}))
        .await
        .unwrap();
}

#[tokio::test]
async fn delete_file() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let write = WriteFileTool;
    write
        .execute(&ctx, json!({"path": "to_delete.txt", "content": "x"}))
        .await
        .unwrap();

    let delete = DeleteFileTool;
    let r = delete
        .execute(&ctx, json!({"path": "to_delete.txt"}))
        .await
        .unwrap();
    assert!(r.ok);

    let read = ReadFileTool;
    let r = read.execute(&ctx, json!({"path": "to_delete.txt"})).await;
    assert!(r.is_err()); // file gone
}

#[tokio::test]
async fn delete_non_empty_dir_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let write = WriteFileTool;
    write
        .execute(&ctx, json!({"path": "dir/inner.txt", "content": "x"}))
        .await
        .unwrap();

    let delete = DeleteFileTool;
    let r = delete.execute(&ctx, json!({"path": "dir"})).await.unwrap();
    assert!(!r.ok);
    assert!(r.output.contains("not empty"));
}

#[tokio::test]
async fn search_files_glob() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let write = WriteFileTool;
    write.execute(&ctx, json!({"path": "a.rs", "content": "x"})).await.unwrap();
    write.execute(&ctx, json!({"path": "b.txt", "content": "x"})).await.unwrap();
    write.execute(&ctx, json!({"path": "c.rs", "content": "x"})).await.unwrap();
    write.execute(&ctx, json!({"path": "nested/d.rs", "content": "x"})).await.unwrap();

    let search = SearchFilesTool;
    let r = search
        .execute(&ctx, json!({"pattern": "**/*.rs"}))
        .await
        .unwrap();
    let matches: Vec<String> = r.data.unwrap()["matches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(matches.contains(&"a.rs".to_string()));
    assert!(matches.contains(&"c.rs".to_string()));
    assert!(matches.contains(&"nested/d.rs".to_string()));
    assert!(!matches.contains(&"b.txt".to_string()));
}

#[tokio::test]
async fn sandbox_blocks_path_escape() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;

    let read = ReadFileTool;
    let r = read.execute(&ctx, json!({"path": "../../etc/passwd"})).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn read_nonexistent_file_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;
    let read = ReadFileTool;
    let r = read.execute(&ctx, json!({"path": "ghost.txt"})).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn write_creates_parent_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = make_ctx(tmp.path().to_path_buf()).await;
    let write = WriteFileTool;
    let r = write
        .execute(&ctx, json!({"path": "a/b/c/file.txt", "content": "deep"}))
        .await
        .unwrap();
    assert!(r.ok);
}
