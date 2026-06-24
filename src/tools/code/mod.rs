//! Code execution tools (Python, JavaScript).

pub mod run_javascript;
pub mod run_python;

#[cfg(test)]
mod tests;

pub fn register_all(registry: &crate::tools::registry::ToolRegistry) {
    registry.register(run_python::RunPythonTool);
    registry.register(run_javascript::RunJavaScriptTool);
}
