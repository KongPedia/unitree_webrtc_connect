use crate::encryption::{
    aes_decrypt, aes_encrypt, generate_aes_key, rsa_encrypt, rsa_load_public_key,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::{Client, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug)]
pub enum SignalingError {
    Http(ReqwestError),
    Crypto(&'static str),
    Json(serde_json::Error),
    Other(String),
}

impl From<ReqwestError> for SignalingError {
    fn from(err: ReqwestError) -> Self {
        SignalingError::Http(err)
    }
}

impl From<serde_json::Error> for SignalingError {
    fn from(err: serde_json::Error) -> Self {
        SignalingError::Json(err)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ConNotifyResponse {
    data1: String,
    data2: Option<i32>,
}

#[derive(Serialize)]
struct ConIngRequest {
    data1: String,
    data2: String,
}

pub fn calc_local_path_ending(data1: &str) -> String {
    let str_arr = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];
    let last_10 = if data1.len() >= 10 {
        &data1[data1.len() - 10..]
    } else {
        data1
    };

    let mut result = String::new();
    let chars: Vec<char> = last_10.chars().collect();
    for chunk in chars.chunks(2) {
        if chunk.len() > 1 {
            let second_char = chunk[1].to_string();
            if let Some(index) = str_arr.iter().position(|&s| s == second_char) {
                result.push_str(&index.to_string());
            }
        }
    }
    result
}

pub fn decrypt_con_notify_data(encrypted_b64: &str) -> Result<String, SignalingError> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes128Gcm, Nonce,
    };

    let key = [
        232, 86, 130, 189, 22, 84, 155, 0, 142, 4, 166, 104, 43, 179, 235, 227u8,
    ];

    let data = BASE64
        .decode(encrypted_b64)
        .map_err(|_| SignalingError::Crypto("Invalid base64 in con_notify_data"))?;

    if data.len() < 28 {
        return Err(SignalingError::Crypto("Decryption failed: input too short"));
    }

    // python logic:
    // tag = data[-16:]
    // nonce = data[-28:-16]
    // ciphertext = data[:-28]
    // AESGCM requires ciphertext + tag appended, which is exactly data[:-28] + data[-16:]
    // Actually, in Rust `aes_gcm`, `decrypt` expects `ciphertext` and `tag` concatenated.
    // So `payload = [ciphertext, tag].concat()`

    let tag_start = data.len() - 16;
    let nonce_start = data.len() - 28;

    let ciphertext = &data[..nonce_start];
    let nonce_bytes = &data[nonce_start..tag_start];
    let tag = &data[tag_start..];

    let mut payload = Vec::with_capacity(ciphertext.len() + tag.len());
    payload.extend_from_slice(ciphertext);
    payload.extend_from_slice(tag);

    let cipher = Aes128Gcm::new(&key.into());
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext_bytes = cipher
        .decrypt(nonce, payload.as_ref())
        .map_err(|_| SignalingError::Crypto("AES-GCM decryption failed for con_notify_data"))?;

    String::from_utf8(plaintext_bytes)
        .map_err(|_| SignalingError::Crypto("Invalid UTF-8 in decrypted con_notify_data"))
}

pub async fn send_sdp_to_local_peer(ip: &str, sdp: &str) -> Result<String, SignalingError> {
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;

    // 1. Try old method
    if let Ok(response) = send_sdp_to_local_peer_old_method(&client, ip, sdp).await {
        return Ok(response);
    }

    // 2. Fallback to new method
    send_sdp_to_local_peer_new_method(&client, ip, sdp).await
}

async fn send_sdp_to_local_peer_old_method(
    client: &Client,
    ip: &str,
    sdp: &str,
) -> Result<String, SignalingError> {
    let url = format!("http://{}:8081/offer", ip);
    let res = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(sdp.to_string())
        .send()
        .await?
        .error_for_status()?;

    let text = res.text().await?;
    Ok(text)
}

async fn send_sdp_to_local_peer_new_method(
    client: &Client,
    ip: &str,
    sdp: &str,
) -> Result<String, SignalingError> {
    let url = format!("http://{}:9991/con_notify", ip);

    // Initial request
    let res = client.post(&url).send().await?.error_for_status()?;
    let encoded_resp = res.text().await?;

    let decoded_resp = BASE64
        .decode(encoded_resp.trim())
        .map_err(|_| SignalingError::Crypto("Base64 decode failed for con_notify response"))?;
    let decoded_str = String::from_utf8(decoded_resp)
        .map_err(|_| SignalingError::Crypto("UTF-8 decode failed for con_notify response"))?;

    let notify_json: ConNotifyResponse = serde_json::from_str(&decoded_str)?;

    let mut data1 = notify_json.data1;
    if let Some(2) = notify_json.data2 {
        data1 = decrypt_con_notify_data(&data1)?;
    }

    if data1.len() < 20 {
        return Err(SignalingError::Crypto("data1 length is too short"));
    }

    // Extract public key pem and path ending
    let pem_len = data1.len() - 10;
    let public_key_pem = &data1[10..pem_len];
    let path_ending = calc_local_path_ending(&data1);
    let url2 = format!("http://{}:9991/con_ing_{}", ip, path_ending);

    // Generate AES key and load RSA
    let aes_key = generate_aes_key();
    let public_key = rsa_load_public_key(public_key_pem).map_err(SignalingError::Crypto)?;

    let req_body = ConIngRequest {
        data1: aes_encrypt(sdp, &aes_key),
        data2: rsa_encrypt(&aes_key, &public_key).map_err(SignalingError::Crypto)?,
    };
    let req_json = serde_json::to_string(&req_body)?;

    let res2 = client
        .post(&url2)
        .header("Content-Type", "application/x-www-form-urlencoded") // Matches python
        .body(req_json)
        .send()
        .await?
        .error_for_status()?;

    let encrypted_resp = res2.text().await?;

    // Decrypt the response
    aes_decrypt(&encrypted_resp, &aes_key).map_err(SignalingError::Crypto)
}
