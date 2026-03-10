use unitree_webrtc_connect_rs::utils::*;

#[test]
fn test_generate_md5() {
    let test_str = "password123";
    let hash = generate_md5(test_str);
    assert_eq!(hash, "482c811da5d5b4bc6d497ffa98491e38");
}

#[test]
fn test_generate_uuid_js_style() {
    let uuid_str = generate_uuid_js_style();
    assert_eq!(uuid_str.len(), 36);
    assert_eq!(uuid_str.chars().nth(14), Some('4'));
    let y_char = uuid_str.chars().nth(19).unwrap();
    assert!(['8', '9', 'a', 'b'].contains(&y_char));
}
