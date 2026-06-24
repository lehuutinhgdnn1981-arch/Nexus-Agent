//! ID generators (UUID v4).

use uuid::Uuid;

/// Sinh UUID v4 mới dạng string.
#[must_use]
pub fn new_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Sinh UUID v4 dạng `Uuid`.
#[must_use]
pub fn new_uuid_value() -> Uuid {
    Uuid::new_v4()
}

/// Sinh ID ngắn (8 ký tự đầu của UUID v4) — dùng cho log/trace-friendly IDs.
#[must_use]
pub fn short_id() -> String {
    let s = Uuid::new_v4().simple().to_string();
    s[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_unique() {
        let a = new_uuid();
        let b = new_uuid();
        assert_ne!(a, b);
        assert_eq!(a.len(), 36);
    }

    #[test]
    fn short_id_length() {
        let s = short_id();
        assert_eq!(s.len(), 8);
    }
}
