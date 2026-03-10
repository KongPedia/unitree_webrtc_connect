use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes256;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use uuid::Uuid;

/// Generates a UUID and returns it as a 32-character hexadecimal string.
pub fn generate_uuid() -> String {
    let bytes = Uuid::new_v4().into_bytes();
    hex::encode(bytes)
}

/// Generates an AES key (which is just a 32-character UUID).
pub fn generate_aes_key() -> String {
    generate_uuid()
}

/// Helper function to pad data to be a multiple of 16 bytes (PKCS7).
fn pad(data: &str) -> Vec<u8> {
    let mut bytes = data.as_bytes().to_vec();
    let block_size = 16;
    let padding = block_size - (bytes.len() % block_size);
    bytes.resize(bytes.len() + padding, padding as u8);
    bytes
}

/// Helper function to remove padding from data (PKCS7).
fn unpad(data: &[u8]) -> Result<String, &'static str> {
    if data.is_empty() {
        return Err("Data is empty");
    }
    let padding = data[data.len() - 1] as usize;
    if padding == 0 || padding > 16 || padding > data.len() {
        return Err("Invalid padding length");
    }
    for i in 0..padding {
        if data[data.len() - 1 - i] as usize != padding {
            return Err("Invalid padding bytes");
        }
    }
    let unpadded = &data[..data.len() - padding];
    String::from_utf8(unpadded.to_vec()).map_err(|_| "Invalid UTF-8 in unpadded data")
}

/// Encrypt the given data using AES-256 ECB mode with PKCS7 padding.
pub fn aes_encrypt(data: &str, key: &str) -> String {
    // Ensure key is 32 bytes for AES-256
    let key_bytes = key.as_bytes();
    if key_bytes.len() != 32 {
        panic!("Key must be exactly 32 bytes for AES-256");
    }

    let cipher = Aes256::new(key_bytes.into());
    let mut padded_data = pad(data);

    // Encrypt block by block (ECB mode)
    for chunk in padded_data.chunks_mut(16) {
        let block = aes::cipher::generic_array::GenericArray::from_mut_slice(chunk);
        cipher.encrypt_block(block);
    }

    // Base64 encode
    BASE64.encode(&padded_data)
}

/// Decrypt the given data using AES-256 ECB mode with PKCS7 padding.
pub fn aes_decrypt(encrypted_data: &str, key: &str) -> Result<String, &'static str> {
    let key_bytes = key.as_bytes();
    if key_bytes.len() != 32 {
        return Err("Key must be exactly 32 bytes for AES-256");
    }

    let mut decoded = BASE64
        .decode(encrypted_data)
        .map_err(|_| "Failed to decode base64")?;

    if decoded.len() % 16 != 0 {
        return Err("Decoded data length is not a multiple of block size");
    }

    let cipher = Aes256::new(key_bytes.into());

    // Decrypt block by block (ECB mode)
    for chunk in decoded.chunks_mut(16) {
        let block = aes::cipher::generic_array::GenericArray::from_mut_slice(chunk);
        cipher.decrypt_block(block);
    }

    unpad(&decoded)
}

/// Load an RSA public key from a base64-encoded DER string.
pub fn rsa_load_public_key(pem_data_base64: &str) -> Result<RsaPublicKey, &'static str> {
    // Clean whitespaces if any
    let cleaned: String = pem_data_base64
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let key_bytes = BASE64
        .decode(cleaned)
        .map_err(|_| "Invalid base64 in public key")?;
    RsaPublicKey::from_public_key_der(&key_bytes).map_err(|_| "Invalid RSA public key DER")
}

/// Encrypt data using RSA PKCS1 v1.5 padding and a given public key.
pub fn rsa_encrypt(data: &str, public_key: &RsaPublicKey) -> Result<String, &'static str> {
    let mut rng = rand::rngs::OsRng;
    let max_chunk_size = public_key.size() - 11;
    let data_bytes = data.as_bytes();

    let mut encrypted_bytes = Vec::new();
    for chunk in data_bytes.chunks(max_chunk_size) {
        let encrypted_chunk = public_key
            .encrypt(&mut rng, Pkcs1v15Encrypt, chunk)
            .map_err(|_| "RSA encryption failed")?;
        encrypted_bytes.extend(encrypted_chunk);
    }

    Ok(BASE64.encode(&encrypted_bytes))
}
