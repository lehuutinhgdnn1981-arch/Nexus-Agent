//! Browser tools — wrappers around BrowserManager + page actions.

pub mod click;
pub mod extract_text;
pub mod navigate;
pub mod screenshot;
pub mod type_text;
pub mod wait;

pub fn register_all(registry: &crate::tools::registry::ToolRegistry) {
    registry.register(navigate::BrowserNavigateTool);
    registry.register(click::BrowserClickTool);
    registry.register(type_text::BrowserTypeTool);
    registry.register(wait::BrowserWaitTool);
    registry.register(extract_text::BrowserExtractTextTool);
    registry.register(screenshot::BrowserScreenshotTool);
}
