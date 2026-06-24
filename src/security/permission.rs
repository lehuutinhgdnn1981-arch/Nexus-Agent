//! Permission levels.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Mức độ phân quyền cho tool.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    /// An toàn — tự động thực thi không cần hỏi.
    Safe,
    /// Cần user approve qua IPC.
    RequiresApproval,
    /// Nguy hiểm — cần approve + warning thêm.
    Dangerous,
}

impl PermissionLevel {
    #[must_use]
    pub fn requires_approval(self) -> bool {
        matches!(self, Self::RequiresApproval | Self::Dangerous)
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::RequiresApproval => "requires_approval",
            Self::Dangerous => "dangerous",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_approval_matrix() {
        assert!(!PermissionLevel::Safe.requires_approval());
        assert!(PermissionLevel::RequiresApproval.requires_approval());
        assert!(PermissionLevel::Dangerous.requires_approval());
    }
}
