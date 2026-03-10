use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct FutureResolver {
    pending_callbacks: HashSet<String>,
    resolved_messages: HashMap<String, Value>,
    chunk_data_storage: HashMap<String, Vec<Vec<u8>>>,
}

impl FutureResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn save_resolve(&mut self, message_type: &str, topic: &str, identifier: Option<&str>) {
        let key = Self::generate_message_key(message_type, topic, identifier);
        self.pending_callbacks.insert(key);
    }

    pub fn has_pending(&self, key: &str) -> bool {
        self.pending_callbacks.contains(key)
    }

    pub fn take_resolved(&mut self, key: &str) -> Option<Value> {
        self.resolved_messages.remove(key)
    }

    pub fn run_resolve_for_topic(&mut self, message: &mut Value) -> Result<Option<String>, String> {
        let message_type = message
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| "Message type missing".to_string())?;

        let topic = message
            .get("topic")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let identifier = extract_identifier(message);
        let key = Self::generate_message_key(message_type, topic, identifier.as_deref());

        if is_chunked_data(message) {
            let (chunk_index, total_chunks, chunk_data) = extract_data_chunk(message)?;
            self.push_chunk(&key, chunk_data);
            if chunk_index < total_chunks {
                return Ok(None);
            }

            let buffers = self
                .chunk_data_storage
                .remove(&key)
                .ok_or_else(|| "Chunk storage missing".to_string())?;
            let merged = self.merge_array_buffers(buffers);

            if let Some(data_obj) = message.get_mut("data").and_then(Value::as_object_mut) {
                data_obj.insert(
                    "data".to_string(),
                    Value::Array(merged.into_iter().map(Value::from).collect()),
                );
            }
        }

        if self.pending_callbacks.remove(&key) {
            self.resolved_messages.insert(key.clone(), message.clone());
            return Ok(Some(key));
        }

        Ok(None)
    }

    pub fn run_resolve_for_topic_for_file(
        &mut self,
        message: &mut Value,
    ) -> Result<Option<String>, String> {
        let message_type = message
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| "Message type missing".to_string())?;
        let topic = message
            .get("topic")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let identifier = extract_identifier(message);
        let key = Self::generate_message_key(message_type, topic, identifier.as_deref());

        let file = message
            .get_mut("info")
            .and_then(Value::as_object_mut)
            .and_then(|info| info.get_mut("file"))
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "Missing info.file".to_string())?;

        let enable_chunking = file
            .get("enable_chunking")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if enable_chunking {
            let chunk_index = file
                .get("chunk_index")
                .and_then(Value::as_u64)
                .ok_or_else(|| "Chunk index is missing".to_string())?;
            let total_chunks = file
                .get("total_chunk_num")
                .and_then(Value::as_u64)
                .ok_or_else(|| "Total chunks is missing".to_string())?;
            if total_chunks == 0 {
                return Err("Total number of chunks cannot be zero".to_string());
            }

            let chunk_data = file
                .get("data")
                .and_then(Value::as_str)
                .ok_or_else(|| "File chunk data missing".to_string())?;
            self.push_chunk(&key, chunk_data.as_bytes().to_vec());

            if chunk_index == total_chunks {
                let buffers = self
                    .chunk_data_storage
                    .remove(&key)
                    .ok_or_else(|| "Chunk storage missing".to_string())?;
                let merged = self.merge_array_buffers(buffers);
                file.insert(
                    "data".to_string(),
                    Value::String(String::from_utf8_lossy(&merged).to_string()),
                );
            } else {
                return Ok(None);
            }
        }

        if self.pending_callbacks.remove(&key) {
            self.resolved_messages.insert(key.clone(), message.clone());
            return Ok(Some(key));
        }

        Ok(None)
    }

    pub fn generate_message_key(
        message_type: &str,
        topic: &str,
        identifier: Option<&str>,
    ) -> String {
        identifier
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("{message_type} $ {topic}"))
    }

    fn push_chunk(&mut self, key: &str, chunk: Vec<u8>) {
        self.chunk_data_storage
            .entry(key.to_string())
            .or_default()
            .push(chunk);
    }

    fn merge_array_buffers(&self, buffers: Vec<Vec<u8>>) -> Vec<u8> {
        let total_length: usize = buffers.iter().map(Vec::len).sum();
        let mut merged_buffer = Vec::with_capacity(total_length);
        for buffer in buffers {
            merged_buffer.extend_from_slice(&buffer);
        }
        merged_buffer
    }
}

fn extract_identifier(message: &Value) -> Option<String> {
    get_nested_field(message, &["data", "uuid"])
        .or_else(|| get_nested_field(message, &["data", "header", "identity", "id"]))
        .or_else(|| get_nested_field(message, &["info", "uuid"]))
        .or_else(|| get_nested_field(message, &["info", "req_uuid"]))
}

fn get_nested_field(message: &Value, path: &[&str]) -> Option<String> {
    let mut current = message;
    for key in path {
        current = current.get(*key)?;
    }

    match current {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn is_chunked_data(message: &Value) -> bool {
    message
        .get("data")
        .and_then(|data| data.get("content_info"))
        .and_then(|content_info| content_info.get("enable_chunking"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn extract_data_chunk(message: &Value) -> Result<(u64, u64, Vec<u8>), String> {
    let content_info = message
        .get("data")
        .and_then(|data| data.get("content_info"))
        .ok_or_else(|| "content_info missing".to_string())?;

    let chunk_index = content_info
        .get("chunk_index")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Chunk index is missing".to_string())?;
    let total_chunks = content_info
        .get("total_chunk_num")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Total number of chunks is missing".to_string())?;
    if total_chunks == 0 {
        return Err("Total number of chunks cannot be zero".to_string());
    }

    let data_chunk = message
        .get("data")
        .and_then(|data| data.get("data"))
        .ok_or_else(|| "Chunk data missing".to_string())?;

    let chunk_bytes = match data_chunk {
        Value::Array(arr) => {
            let mut bytes = Vec::with_capacity(arr.len());
            for v in arr {
                let num = v
                    .as_u64()
                    .ok_or_else(|| "Chunk array must contain integers".to_string())?;
                let b = u8::try_from(num).map_err(|_| "Chunk byte out of range".to_string())?;
                bytes.push(b);
            }
            bytes
        }
        Value::String(s) => s.as_bytes().to_vec(),
        _ => return Err("Unsupported chunk data type".to_string()),
    };

    Ok((chunk_index, total_chunks, chunk_bytes))
}
