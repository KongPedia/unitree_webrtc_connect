use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes128Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use unitree_webrtc_connect_rs::signaling::{calc_local_path_ending, decrypt_con_notify_data};

#[test]
fn test_calc_local_path_ending_extracts_expected_digits() {
    let sample = "1234567A8B";
    assert_eq!(calc_local_path_ending(sample), "01");
}

#[test]
fn test_decrypt_con_notify_data_invalid_base64_returns_error() {
    let result = decrypt_con_notify_data("@@@not_base64@@@");
    assert!(result.is_err());
}

#[test]
fn test_decrypt_con_notify_data_too_short_returns_error() {
    let short_payload = BASE64.encode([1u8, 2, 3, 4, 5]);
    let result = decrypt_con_notify_data(&short_payload);
    assert!(result.is_err());
}

#[test]
fn test_decrypt_con_notify_data_valid_payload() {
    let key = [
        232, 86, 130, 189, 22, 84, 155, 0, 142, 4, 166, 104, 43, 179, 235, 227u8,
    ];
    let nonce_bytes = [1u8, 3, 5, 7, 9, 11, 13, 15, 2, 4, 6, 8];
    let plaintext = b"hello-con-notify";

    let cipher = Aes128Gcm::new(&key.into());
    let nonce = Nonce::from_slice(&nonce_bytes);
    let encrypted = cipher
        .encrypt(nonce, plaintext.as_ref())
        .expect("encryption should work in test");

    let tag_start = encrypted.len() - 16;
    let ciphertext = &encrypted[..tag_start];
    let tag = &encrypted[tag_start..];

    let mut python_style_payload = Vec::new();
    python_style_payload.extend_from_slice(ciphertext);
    python_style_payload.extend_from_slice(&nonce_bytes);
    python_style_payload.extend_from_slice(tag);

    let encoded = BASE64.encode(python_style_payload);
    let decrypted = decrypt_con_notify_data(&encoded).expect("decryption should succeed");

    assert_eq!(decrypted, "hello-con-notify");
}
