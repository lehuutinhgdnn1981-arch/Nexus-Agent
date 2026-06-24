//! Shell tools.

pub mod blacklist;
pub mod run_command;

#[cfg(test)]
mod tests;

pub use blacklist::ShellBlacklist;
pub use run_command::RunCommandTool;

pub fn register_all(registry: &crate::tools::registry::ToolRegistry) {
    registry.register(run_command::RunCommandTool::new());
}
