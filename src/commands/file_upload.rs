//! File upload command — read file content from disk for AI to read.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct FileContent {
    pub filename: String,
    pub path: String,
    pub content: String,
    pub size: usize,
    pub is_binary: bool,
    pub language: String,
}

#[tauri::command]
pub async fn read_file_for_chat(path: String) -> Result<FileContent, super::IpcError> {
    let path_buf = PathBuf::from(&path);
    let filename = path_buf
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());

    let metadata = std::fs::metadata(&path_buf)
        .map_err(|e| super::IpcError {
            code: "io_error".into(),
            message: format!("Cannot read file: {e}"),
        })?;

    let size = metadata.len() as usize;

    // Check if binary
    let bytes = std::fs::read(&path_buf)
        .map_err(|e| super::IpcError {
            code: "io_error".into(),
            message: format!("Cannot read file: {e}"),
        })?;

    let is_binary = bytes.iter().take(1024).any(|&b| b == 0);

    let content = if is_binary {
        format!("[Binary file: {filename}, {size} bytes — cannot display]")
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };

    // Detect language from extension
    let language = match path_buf.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("py") => "python",
        Some("js") | Some("jsx") => "javascript",
        Some("ts") | Some("tsx") => "typescript",
        Some("go") => "go",
        Some("java") => "java",
        Some("c") | Some("h") => "c",
        Some("cpp") | Some("hpp") => "cpp",
        Some("rb") => "ruby",
        Some("php") => "php",
        Some("sh") | Some("bash") => "bash",
        Some("sql") => "sql",
        Some("html") => "html",
        Some("css") => "css",
        Some("json") => "json",
        Some("xml") => "xml",
        Some("yaml") | Some("yml") => "yaml",
        Some("toml") => "toml",
        Some("md") => "markdown",
        Some("txt") => "text",
        _ => "text",
    };

    Ok(FileContent {
        filename,
        path,
        content,
        size,
        is_binary,
        language,
    })
}
