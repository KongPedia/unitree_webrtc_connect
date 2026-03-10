use serde_json::{json, Value};

const AUDIO_HUB_TOPIC: &str = "rt/api/audiohub/request";
const API_GET_AUDIO_LIST: i64 = 1001;
const API_SELECT_START_PLAY: i64 = 1002;
const API_PAUSE: i64 = 1003;
const API_UNSUSPEND: i64 = 1004;
const API_SET_PLAY_MODE: i64 = 1007;
const API_SELECT_RENAME: i64 = 1008;
const API_SELECT_DELETE: i64 = 1009;
const API_GET_PLAY_MODE: i64 = 1010;
const API_UPLOAD_AUDIO_FILE: i64 = 2001;
const API_ENTER_MEGAPHONE: i64 = 4001;
const API_EXIT_MEGAPHONE: i64 = 4002;
const API_UPLOAD_MEGAPHONE: i64 = 4003;

pub struct WebRTCAudioHub;

impl Default for WebRTCAudioHub {
    fn default() -> Self {
        Self::new()
    }
}

impl WebRTCAudioHub {
    pub fn new() -> Self {
        Self
    }

    pub fn get_audio_list_request() -> Value {
        build_request(API_GET_AUDIO_LIST, json!({}))
    }

    pub fn play_by_uuid_request(unique_id: &str) -> Value {
        build_request(API_SELECT_START_PLAY, json!({ "unique_id": unique_id }))
    }

    pub fn pause_request() -> Value {
        build_request(API_PAUSE, json!({}))
    }

    pub fn resume_request() -> Value {
        build_request(API_UNSUSPEND, json!({}))
    }

    pub fn set_play_mode_request(play_mode: &str) -> Value {
        build_request(API_SET_PLAY_MODE, json!({ "play_mode": play_mode }))
    }

    pub fn rename_record_request(unique_id: &str, new_name: &str) -> Value {
        build_request(
            API_SELECT_RENAME,
            json!({ "unique_id": unique_id, "new_name": new_name }),
        )
    }

    pub fn delete_record_request(unique_id: &str) -> Value {
        build_request(API_SELECT_DELETE, json!({ "unique_id": unique_id }))
    }

    pub fn get_play_mode_request() -> Value {
        build_request(API_GET_PLAY_MODE, json!({}))
    }

    pub fn enter_megaphone_request() -> Value {
        build_request(API_ENTER_MEGAPHONE, json!({}))
    }

    pub fn exit_megaphone_request() -> Value {
        build_request(API_EXIT_MEGAPHONE, json!({}))
    }

    pub fn upload_audio_file_requests(
        file_name: &str,
        file_size: usize,
        file_md5: &str,
        base64_content: &str,
        block_size: usize,
    ) -> Vec<Value> {
        split_and_build_upload_requests(
            API_UPLOAD_AUDIO_FILE,
            file_name,
            file_size,
            file_md5,
            base64_content,
            block_size,
        )
    }

    pub fn upload_megaphone_requests(base64_content: &str, block_size: usize) -> Vec<Value> {
        let chunks = split_chunks(base64_content, block_size);
        let total = chunks.len();

        chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| {
                build_request(
                    API_UPLOAD_MEGAPHONE,
                    json!({
                        "current_block_size": chunk.len(),
                        "block_content": chunk,
                        "current_block_index": index + 1,
                        "total_block_number": total,
                    }),
                )
            })
            .collect()
    }
}

fn build_request(api_id: i64, parameter: Value) -> Value {
    json!({
        "topic": AUDIO_HUB_TOPIC,
        "api_id": api_id,
        "parameter": parameter,
    })
}

fn split_and_build_upload_requests(
    api_id: i64,
    file_name: &str,
    file_size: usize,
    file_md5: &str,
    base64_content: &str,
    block_size: usize,
) -> Vec<Value> {
    let chunks = split_chunks(base64_content, block_size);
    let total = chunks.len();

    chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            build_request(
                api_id,
                json!({
                    "file_name": file_name,
                    "file_type": "wav",
                    "file_size": file_size,
                    "current_block_index": index + 1,
                    "total_block_number": total,
                    "block_content": chunk,
                    "current_block_size": chunk.len(),
                    "file_md5": file_md5,
                }),
            )
        })
        .collect()
}

fn split_chunks(content: &str, block_size: usize) -> Vec<String> {
    if block_size == 0 {
        return vec![content.to_string()];
    }

    content
        .as_bytes()
        .chunks(block_size)
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect()
}
