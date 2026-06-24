//! JSON helpers.

use serde_json::Value;

/// Lấy value tại `path` (ví dụ "user.address.city") từ JSON object.
#[must_use]
pub fn get_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }
        current = current.get(segment)?;
    }
    Some(current)
}

/// Merge 2 JSON object (b override a). Trả về object mới.
#[must_use]
pub fn merge_objects(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Object(a_obj), Value::Object(b_obj)) => {
            let mut result = a_obj.clone();
            for (k, v) in b_obj {
                result.insert(k.clone(), v.clone());
            }
            Value::Object(result)
        }
        // Nếu không phải 2 object, b thắng.
        (_, b_value) => b_value.clone(),
    }
}

/// Pretty-print JSON với 2-space indent.
#[must_use]
pub fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_path_nested() {
        let v = json!({ "user": { "address": { "city": "Hanoi" } } });
        let city = get_path(&v, "user.address.city");
        assert_eq!(city, Some(&json!("Hanoi")));
    }

    #[test]
    fn get_path_missing() {
        let v = json!({ "a": 1 });
        assert!(get_path(&v, "b.c").is_none());
    }

    #[test]
    fn merge_overrides() {
        let a = json!({ "x": 1, "y": 2 });
        let b = json!({ "y": 3, "z": 4 });
        let merged = merge_objects(&a, &b);
        assert_eq!(merged, json!({ "x": 1, "y": 3, "z": 4 }));
    }
}
