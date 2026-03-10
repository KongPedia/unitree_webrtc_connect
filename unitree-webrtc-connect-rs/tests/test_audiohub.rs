use unitree_webrtc_connect_rs::audiohub::WebRTCAudioHub;

#[test]
fn test_audiohub_basic_requests() {
    let _hub = WebRTCAudioHub::new();

    let get_list = WebRTCAudioHub::get_audio_list_request();
    assert_eq!(get_list["topic"], "rt/api/audiohub/request");
    assert_eq!(get_list["api_id"], 1001);

    let play = WebRTCAudioHub::play_by_uuid_request("uuid-1");
    assert_eq!(play["api_id"], 1002);
    assert_eq!(play["parameter"]["unique_id"], "uuid-1");

    let pause = WebRTCAudioHub::pause_request();
    assert_eq!(pause["api_id"], 1003);

    let resume = WebRTCAudioHub::resume_request();
    assert_eq!(resume["api_id"], 1004);

    let mode = WebRTCAudioHub::set_play_mode_request("list_loop");
    assert_eq!(mode["api_id"], 1007);
    assert_eq!(mode["parameter"]["play_mode"], "list_loop");
}

#[test]
fn test_audiohub_upload_request_chunking() {
    let requests = WebRTCAudioHub::upload_audio_file_requests("sample", 12, "md5", "ABCDEFGHIJ", 4);
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0]["api_id"], 2001);
    assert_eq!(requests[0]["parameter"]["current_block_index"], 1);
    assert_eq!(requests[0]["parameter"]["total_block_number"], 3);
    assert_eq!(requests[2]["parameter"]["block_content"], "IJ");

    let megaphone = WebRTCAudioHub::upload_megaphone_requests("ABCDEFG", 3);
    assert_eq!(megaphone.len(), 3);
    assert_eq!(megaphone[0]["api_id"], 4003);
    assert_eq!(megaphone[1]["parameter"]["current_block_index"], 2);
}
