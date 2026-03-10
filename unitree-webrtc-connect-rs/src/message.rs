use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ParsedMessage {
    pub is_lidar: bool,
    pub json_data: Value,
    pub binary_data: Vec<u8>,
}

pub fn parse_array_buffer(buffer: &[u8]) -> Result<ParsedMessage, String> {
    if buffer.len() < 4 {
        return Err("Buffer too small to contain header".to_string());
    }

    let header_1 = u16::from_le_bytes([buffer[0], buffer[1]]);
    let header_2 = u16::from_le_bytes([buffer[2], buffer[3]]);

    if header_1 == 2 && header_2 == 0 {
        // lidar path
        parse_for_lidar(&buffer[4..])
    } else {
        // normal path
        parse_for_normal(buffer)
    }
}

fn parse_for_normal(buffer: &[u8]) -> Result<ParsedMessage, String> {
    if buffer.len() < 4 {
        return Err("Buffer too small for normal header".to_string());
    }
    let header_length = u16::from_le_bytes([buffer[0], buffer[1]]) as usize;

    if buffer.len() < 4 + header_length {
        return Err("Buffer too small for normal json data".to_string());
    }

    let json_bytes = &buffer[4..4 + header_length];
    let binary_data = buffer[4 + header_length..].to_vec();

    let json_data: Value =
        serde_json::from_slice(json_bytes).map_err(|e| format!("JSON parsing error: {}", e))?;

    Ok(ParsedMessage {
        is_lidar: false,
        json_data,
        binary_data,
    })
}

fn parse_for_lidar(buffer: &[u8]) -> Result<ParsedMessage, String> {
    if buffer.len() < 8 {
        return Err("Buffer too small for lidar header".to_string());
    }
    let header_length = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;

    if buffer.len() < 8 + header_length {
        return Err("Buffer too small for lidar json data".to_string());
    }

    let json_bytes = &buffer[8..8 + header_length];
    let binary_data = buffer[8 + header_length..].to_vec();

    let json_data: Value =
        serde_json::from_slice(json_bytes).map_err(|e| format!("JSON parsing error: {}", e))?;

    Ok(ParsedMessage {
        is_lidar: true,
        json_data,
        binary_data,
    })
}
