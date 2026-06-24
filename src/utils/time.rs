//! Time helpers.

use chrono::{DateTime, Utc};

/// Trả về timestamp hiện tại tính bằng giây Unix (i64).
#[must_use]
pub fn now_ts() -> i64 {
    Utc::now().timestamp()
}

/// Trả về timestamp hiện tại tính bằng mili-giây Unix (i64).
#[must_use]
pub fn now_ts_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// Convert i64 Unix seconds → `DateTime<Utc>`.
#[must_use]
pub fn from_ts(ts: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(ts, 0).unwrap_or_default()
}

/// Parse ISO-8601 string → `DateTime<Utc>`. Trả về None nếu parse fail.
pub fn parse_iso(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Format `DateTime<Utc>` thành ISO-8601 string.
#[must_use]
pub fn to_iso(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Format relative time (vd: "2 minutes ago") — rút gọn tiếng Anh.
#[must_use]
pub fn relative(ts: i64) -> String {
    let now = now_ts();
    let delta = now.saturating_sub(ts);
    if delta < 60 {
        format!("{delta}s ago")
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3600)
    } else {
        format!("{}d ago", delta / 86_400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_ts_is_recent() {
        let t = now_ts();
        assert!(t > 1_700_000_000);
    }

    #[test]
    fn parse_iso_roundtrip() {
        let dt = Utc::now();
        let s = to_iso(&dt);
        let parsed = parse_iso(&s);
        assert!(parsed.is_some());
    }

    #[test]
    fn relative_formats() {
        assert_eq!(relative(now_ts()), "0s ago");
        assert_eq!(relative(now_ts() - 120), "2m ago");
        assert_eq!(relative(now_ts() - 7200), "2h ago");
    }
}
