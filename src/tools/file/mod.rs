//! File tools (full implementation ở Phase 5).

pub mod append_file;
pub mod copy_file;
pub mod create_directory;
pub mod delete_file;
pub mod list_directory;
pub mod move_file;
pub mod read_file;
pub mod search_files;
pub mod write_file;

#[cfg(test)]
mod tests;

/// Register toàn bộ file tools vào registry.
pub fn register_all(registry: &crate::tools::registry::ToolRegistry) {
    registry.register(read_file::ReadFileTool);
    registry.register(write_file::WriteFileTool);
    registry.register(append_file::AppendFileTool);
    registry.register(delete_file::DeleteFileTool);
    registry.register(move_file::MoveFileTool);
    registry.register(copy_file::CopyFileTool);
    registry.register(list_directory::ListDirectoryTool);
    registry.register(search_files::SearchFilesTool);
    registry.register(create_directory::CreateDirectoryTool);
}
