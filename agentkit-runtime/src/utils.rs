use serde_json::{Value, json};

fn truncate_utf8_to_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = s[..end].to_string();
    out.push_str("\n... [output truncated]");
    out
}

pub fn apply_output_limit(payload: Value, max_bytes: usize) -> Value {
    let serialized = payload.to_string();
    let truncated = serialized.len() > max_bytes;
    let limited_payload = if truncated {
        Value::String(truncate_utf8_to_bytes(&serialized, max_bytes))
    } else {
        payload
    };

    let mut obj = match limited_payload {
        Value::Object(map) => Value::Object(map),
        other => json!({"value": other}),
    };

    if let Some(map) = obj.as_object_mut() {
        map.insert("truncated".to_string(), Value::Bool(truncated));
        map.insert("max_bytes".to_string(), json!(max_bytes));
    }
    obj
}
