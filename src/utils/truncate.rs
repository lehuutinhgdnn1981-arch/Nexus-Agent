//! Output truncation helpers (cho shell / code execution tools).

use std::time::Duration;

/// Kết quả truncate: giữ lại phần đầu + đuôi nếu quá dài.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TruncatedOutput {
    pub content: String,
    pub original_size: usize,
    pub truncated: bool,
}

/// Cắt output theo số byte tối đa (default 256 KB).
/// Nếu cần cắt, giữ `head_bytes/2` đầu + dòng separator + `tail_bytes/2` cuối.
#[must_use]
pub fn truncate_string(input: &str, max_bytes: usize) -> TruncatedOutput {
    let original_size = input.len();
    if original_size <= max_bytes {
        return TruncatedOutput {
            content: input.to_string(),
            original_size,
            truncated: false,
        };
    }

    // Cắt theo char boundary để không cắt giữa UTF-8 multi-byte char.
    let half = max_bytes / 2;
    let head = input.chars().take_while(|c| c.len_utf8() <= half).collect::<String>();
    let head = if head.len() > half {
        head.chars().take(half.max(1)).collect()
    } else {
        head
    };

    // Tính tail: lấy `half` byte cuối cùng trên char boundary.
    let tail_start = original_size.saturating_sub(half);
    let tail = input[tail_start..].chars().collect::<String>();

    let separator =
        "\n...[truncated]...\n".to_string();
    let content = format!("{head}{separator}{tail}");

    TruncatedOutput {
        content,
        original_size,
        truncated: true,
    }
}

/// Truncate theo KB (1 KB = 1024 byte).
#[must_use]
pub fn truncate_kb(input: &str, max_kb: usize) -> TruncatedOutput {
    truncate_string(input, max_kb.saturating_mul(1024))
}

/// Format `Duration` thành chuỗi human-readable (vd: "1.23s", "456ms").
#[must_use]
pub fn format_duration(d: Duration) -> String {
    let total_ms = d.as_millis();
    if total_ms < 1000 {
        format!("{total_ms}ms")
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_truncation_when_under_limit() {
        let out = truncate_string("hello", 100);
        assert!(!out.truncated);
        assert_eq!(out.content, "hello");
    }

    #[test]
    fn truncates_long_input() {
        let input = "x".repeat(2000);
        let out = truncate_string(&input, 100);
        assert!(out.truncated);
        assert!(out.content.contains("[truncated]"));
        assert!(out.content.len() < 2000);
    }

    #[test]
    fn truncates_utf8_safely() {
        let input = "á".repeat(500); // mỗi char 2 byte
        let out = truncate_string(&input, 50);
        assert!(out.truncated);
        // Phải decode được UTF-8 hợp lệ
        assert!(std::str::from_utf8(out.content.as_bytes()).is_ok());
    }

    #[test]
    fn format_duration_works() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
    }
}
