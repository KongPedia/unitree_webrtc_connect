use serde_json::Value;

pub fn integer_to_hex_string(error_code: i64) -> Result<String, String> {
    if error_code < 0 {
        return Err("Input must be non-negative".to_string());
    }
    Ok(format!("{:X}", error_code))
}

pub fn get_error_code_text(error_source: i64, error_code: &str) -> String {
    match (error_source, error_code) {
        (100, "1") => "DDS message timeout".to_string(),
        (100, "10") => "Battery communication error".to_string(),
        (300, "1") => "Overcurrent".to_string(),
        _ => format!("{error_source}-{error_code}"),
    }
}

pub fn get_error_source_text(error_source: i64) -> String {
    match error_source {
        100 => "Communication firmware malfunction".to_string(),
        300 => "Motor malfunction".to_string(),
        400 => "Radar malfunction".to_string(),
        _ => error_source.to_string(),
    }
}

pub fn handle_error(message: &Value) -> Vec<String> {
    let mut lines = Vec::new();

    let Some(data) = message.get("data").and_then(Value::as_array) else {
        return lines;
    };

    for error in data {
        let Some(items) = error.as_array() else {
            continue;
        };

        if items.len() != 3 {
            continue;
        }

        let timestamp = items[0].as_i64().unwrap_or_default();
        let error_source = items[1].as_i64().unwrap_or_default();
        let error_code_int = items[2].as_i64().unwrap_or_default();

        let error_code_hex = match integer_to_hex_string(error_code_int) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let error_code_text = get_error_code_text(error_source, &error_code_hex);
        let error_source_text = get_error_source_text(error_source);

        lines.push(format!(
            "time={timestamp}, source={error_source_text}, code={error_code_text}"
        ));
    }

    lines
}
