pub fn extract_json(reply: &str) -> Result<serde_json::Value, String> {
    let trimmed = reply.trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return Ok(v);
    }
    // Scan for the first balanced `{...}`, ignoring braces inside strings —
    // shader source is full of them.
    let bytes: Vec<char> = trimmed.chars().collect();
    let start = bytes.iter().position(|c| *c == '{').ok_or_else(|| {
        format!(
            "the reply was not JSON: {}",
            trimmed.chars().take(120).collect::<String>()
        )
    })?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let text: String = bytes[start..=i].iter().collect();
                    return serde_json::from_str(&text)
                        .map_err(|e| format!("the reply was not valid JSON: {e}"));
                }
            }
            _ => {}
        }
    }
    Err("the reply had an unterminated JSON object — the model may have been cut off".into())
}

