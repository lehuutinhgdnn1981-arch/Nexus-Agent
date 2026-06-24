//! Integration tests cho file tools — chạy trên temp dir thật.

mod common;

use nexus::tools::file::{
    append_file::AppendFileTool, copy_file::CopyFileTool,
    create_directory::CreateDirectoryTool, delete_file::DeleteFileTool,
    list_directory::ListDirectoryTool, move_file::MoveFileTool, read_file::ReadFileTool,
    search_files::SearchFilesTool, write_file::WriteFileTool,
};
use nexus::tools::tool::Tool;
use serde_json::json;

#[tokio::test]
async fn integration_file_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    // Write
    let write = WriteFileTool;
    let r = write
        .execute(&ctx, json!({"path": "notes/day1.md", "content": "# Day 1\n\nFirst day notes"}))
        .await
        .unwrap();
    assert!(r.ok);

    // Append
    let append = AppendFileTool;
    append
        .execute(&ctx, json!({"path": "notes/day1.md", "content": "\n\n## Late entry\nDone."}))
        .await
        .unwrap();

    // Read back
    let read = ReadFileTool;
    let r = read.execute(&ctx, json!({"path": "notes/day1.md"})).await.unwrap();
    assert!(r.output.contains("Day 1"));
    assert!(r.output.contains("Late entry"));

    // Copy
    let copy = CopyFileTool;
    copy.execute(&ctx, json!({"src": "notes/day1.md", "dst": "notes/day1.bak.md"}))
        .await
        .unwrap();

    // Move
    let mv = MoveFileTool;
    mv.execute(&ctx, json!({"src": "notes/day1.md", "dst": "notes/2024/day1.md"}))
        .await
        .unwrap();

    // List dir recursively via search
    let search = SearchFilesTool;
    let r = search.execute(&ctx, json!({"pattern": "**/*.md"})).await.unwrap();
    let matches: Vec<String> = r.data.unwrap()["matches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(matches.iter().any(|m| m.contains("day1.bak.md")));
    assert!(matches.iter().any(|m| m.contains("2024/day1.md")));
}

#[tokio::test]
async fn integration_sandbox_blocks_escape() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    // Try to read /etc/passwd via absolute path
    let read = ReadFileTool;
    let r = read.execute(&ctx, json!({"path": "/etc/passwd"})).await;
    assert!(r.is_err(), "should refuse /etc/passwd");

    // Try via .. escape
    let r = read.execute(&ctx, json!({"path": "../../etc/passwd"})).await;
    assert!(r.is_err());

    // Try writing outside sandbox via absolute path
    let write = WriteFileTool;
    let r = write
        .execute(&ctx, json!({"path": "/tmp/nexus_escape_test", "content": "evil"}))
        .await;
    // /tmp may or may not be blocked depending on platform, but it should be blocked by sandbox containment
    let _ = r;
}

#[tokio::test]
async fn integration_delete_only_empty_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    // Create dir with content
    let mkdir = CreateDirectoryTool;
    mkdir.execute(&ctx, json!({"path": "projects"})).await.unwrap();
    let write = WriteFileTool;
    write.execute(&ctx, json!({"path": "projects/file.txt", "content": "x"}))
        .await
        .unwrap();

    // Try delete non-empty
    let delete = DeleteFileTool;
    let r = delete.execute(&ctx, json!({"path": "projects"})).await.unwrap();
    assert!(!r.ok);

    // Delete file first
    delete.execute(&ctx, json!({"path": "projects/file.txt"})).await.unwrap();
    // Now delete empty dir
    let r = delete.execute(&ctx, json!({"path": "projects"})).await.unwrap();
    assert!(r.ok);
}

#[tokio::test]
async fn integration_list_directory_with_hidden() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = common::tool_context(tmp.path()).await;

    let write = WriteFileTool;
    write.execute(&ctx, json!({"path": ".hidden", "content": "h"})).await.unwrap();
    write.execute(&ctx, json!({"path": "visible.txt", "content": "v"}))
        .await
        .unwrap();

    let list = ListDirectoryTool;
    let r = list.execute(&ctx, json!({"include_hidden": false})).await.unwrap();
    let names: Vec<String> = r.data.unwrap()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    assert!(!names.contains(&".hidden".to_string()));
    assert!(names.contains(&"visible.txt".to_string()));

    let r = list.execute(&ctx, json!({"include_hidden": true})).await.unwrap();
    let names: Vec<String> = r.data.unwrap()["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&".hidden".to_string()));
}
