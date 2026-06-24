//! Filesystem sandbox.
//!
//! Sandbox root = `workspace_root()` (từ `config::paths`).
//! Mọi path input từ tool phải được resolve qua `Sandbox::resolve()` để đảm bảo
//! path kết quả nằm bên trong workspace root.

use std::path::{Path, PathBuf};

use crate::error::SecurityError;
use crate::error::Result;

use crate::config::paths::workspace_root;

/// Blocked absolute paths (Unix + Windows).
const BLOCKED_UNIX: &[&str] = &["/etc", "/sys", "/proc", "/boot", "/dev", "/root"];
const BLOCKED_WIN: &[&str] = &[
    "C:\\Windows",
    "C:\\System32",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
];

/// Sandbox filesystem cho file tools.
#[derive(Debug, Clone)]
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// Tạo sandbox với root mặc định (`~/<data_dir>/workspace`).
    #[must_use]
    pub fn new_default() -> Self {
        Self {
            root: workspace_root(),
        }
    }

    /// Tạo sandbox với root tùy chỉnh (dùng cho test).
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Lấy root path.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve một path input (có thể tương đối hoặc tuyệt đối) thành
    /// path tuyệt đối **bên trong** sandbox. Trả về `SandboxViolation` nếu path
    /// thoát khỏi sandbox.
    ///
    /// Quy tắc:
    /// 1. Path tương đối → join với root.
    /// 2. Path tuyệt đối — phải nằm bên trong root, KHÔNG được là blocked path.
    /// 3. Canonicalize path (nếu file tồn tại). Nếu path không tồn tại, vẫn
    ///    phải check parent chain để chống symlink escape.
    pub fn resolve(&self, input: &str) -> Result<PathBuf> {
        let candidate = if Path::new(input).is_absolute() {
            PathBuf::from(input)
        } else {
            self.root.join(input)
        };

        // 1. Blocked path check
        if is_blocked(&candidate) {
            return Err(SecurityError::BlockedSystemPath(input.to_string()).into());
        }

        // 2. Normalize path (không require tồn tại)
        let normalized = normalize_path(&candidate);

        // 3. Nếu path tồn tại, canonicalize để resolve symlinks
        let resolved = match std::fs::canonicalize(&normalized) {
            Ok(canon) => canon,
            Err(_) => normalized,
        };

        // 4. Check workspace containment
        if !resolved.starts_with(&self.root) {
            return Err(SecurityError::SandboxViolation(input.to_string()).into());
        }

        // 5. Re-check blocked sau canonicalize (chống symlink → /etc)
        if is_blocked(&resolved) {
            return Err(SecurityError::BlockedSystemPath(input.to_string()).into());
        }

        Ok(resolved)
    }

    /// Resolve + đảm bảo parent directory tồn tại (tạo nếu cần).
    pub fn resolve_with_parents(&self, input: &str) -> Result<PathBuf> {
        let p = self.resolve(input)?;
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(p)
    }

    /// Kiểm tra path có nằm trong sandbox không (không raise error).
    pub fn contains(&self, input: &str) -> bool {
        self.resolve(input).is_ok()
    }
}

/// Normalize path: resolve `.` và `..` mà không cần file tồn tại.
fn normalize_path(p: &Path) -> PathBuf {
    let mut stack: Vec<std::path::Component<'_>> = Vec::new();
    for component in p.components() {
        match component {
            std::path::Component::ParentDir => {
                // Pop last if not root
                if let Some(last) = stack.last() {
                    if matches!(
                        last,
                        std::path::Component::Normal(_) | std::path::Component::CurDir
                    ) {
                        stack.pop();
                    }
                }
            }
            std::path::Component::CurDir => {} // skip
            other => stack.push(other),
        }
    }
    let mut result = PathBuf::new();
    for c in stack {
        result.push(c.as_os_str());
    }
    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}

/// Kiểm tra path có match blocked list không (case-insensitive trên Windows).
fn is_blocked(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    let path_str_lower = path_str.to_lowercase();

    #[cfg(unix)]
    {
        for blocked in BLOCKED_UNIX {
            if path_str == *blocked || path_str.starts_with(&format!("{blocked}/")) {
                return true;
            }
        }
    }

    #[cfg(windows)]
    {
        for blocked in BLOCKED_WIN {
            let blocked_lower = blocked.to_lowercase();
            if path_str_lower == blocked_lower
                || path_str_lower.starts_with(&format!("{blocked_lower}\\"))
            {
                return true;
            }
        }
    }

    // Trên mọi platform: cũng check cả 2 list để chống path type confusion
    for blocked in BLOCKED_WIN.iter().chain(BLOCKED_UNIX.iter()) {
        let blocked_lower = blocked.to_lowercase();
        if path_str_lower == blocked_lower
            || path_str_lower.starts_with(&format!("{blocked_lower}\\"))
            || path_str_lower.starts_with(&format!("{blocked_lower}/"))
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sandbox() -> (tempfile::TempDir, Sandbox) {
        let tmp = tempfile::tempdir().unwrap();
        let sandbox = Sandbox::new(tmp.path().to_path_buf());
        (tmp, sandbox)
    }

    #[test]
    fn relative_path_resolved_against_root() {
        let (_tmp, sb) = make_sandbox();
        let p = sb.resolve("foo/bar.txt").unwrap();
        assert!(p.starts_with(sb.root()));
    }

    #[test]
    fn parent_dir_escape_rejected() {
        let (_tmp, sb) = make_sandbox();
        let result = sb.resolve("../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn blocked_paths_rejected() {
        let (_tmp, sb) = make_sandbox();
        assert!(sb.resolve("/etc/passwd").is_err());
        assert!(sb.resolve("/sys/kernel").is_err());
        assert!(sb.resolve("/proc/self").is_err());
        assert!(sb.resolve("/boot/vmlinuz").is_err());
    }

    #[test]
    fn windows_blocked_paths_rejected() {
        let (_tmp, sb) = make_sandbox();
        // Path tuyệt đối không trong sandbox → reject
        assert!(sb.resolve("C:\\Windows\\System32").is_err());
    }

    #[test]
    fn absolute_inside_sandbox_ok() {
        let (tmp, sb) = make_sandbox();
        let abs = tmp.path().join("hello.txt");
        let resolved = sb.resolve(abs.to_str().unwrap());
        assert!(resolved.is_ok());
    }

    #[test]
    fn dot_normalized() {
        let (_tmp, sb) = make_sandbox();
        let p = sb.resolve("./foo/./bar.txt").unwrap();
        let expected = sb.root().join("foo").join("bar.txt");
        assert_eq!(p, expected);
    }
}
