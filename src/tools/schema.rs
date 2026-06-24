//! Tool schema (JSON Schema cho LLM tool calling).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    /// JSON Schema for parameters (object type).
    pub parameters: serde_json::Value,
}

impl ToolSchema {
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }

    /// Helper: build schema với properties (name → type, description) và required list.
    #[must_use]
    pub fn object(
        name: &str,
        description: &str,
        properties: Vec<(&str, &str, &str, bool)>,
    ) -> Self {
        let mut props = serde_json::Map::new();
        let mut required: Vec<serde_json::Value> = Vec::new();
        for (pname, ptype, pdesc, is_required) in properties {
            props.insert(
                pname.to_string(),
                serde_json::json!({
                    "type": ptype,
                    "description": pdesc,
                }),
            );
            if is_required {
                required.push(serde_json::json!(pname));
            }
        }
        let parameters = serde_json::json!({
            "type": "object",
            "properties": props,
            "required": required,
        });
        Self::new(name, description, parameters)
    }
}
