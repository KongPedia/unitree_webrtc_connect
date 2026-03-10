use unitree_webrtc_core_rs::encryption::*;

#[test]
fn test_generate_aes_key_has_expected_length() {
    let key = generate_aes_key();
    assert_eq!(key.len(), 32);
}

#[test]
fn test_aes_encrypt_decrypt() {
    let key = "26a663562a6f4dfbbbbf2b50c1a278cb";
    let message = "Hello, world! This is a longer test to span blocks.";

    let encrypted = aes_encrypt(message, key);
    let decrypted = aes_decrypt(&encrypted, key).unwrap();
    assert_eq!(message, decrypted);
}

#[test]
fn test_aes_decrypt_with_invalid_key_length() {
    let key = "26a663562a6f4dfbbbbf2b50c1a278cb";
    let encrypted = aes_encrypt("hello", key);
    let invalid_key = "short_key";

    let result = aes_decrypt(&encrypted, invalid_key);
    assert!(result.is_err());
}

#[test]
fn test_rsa_encrypt() {
    // Sample 2048-bit RSA key base64 (without headers)
    let public_key_base64 = "\
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAnOc1sgpzL4GTVp9/oQ0H\
D7eeAO2GJUABfjX3TitgXiXN1Ktn2WLsLrtAiIuj3OrrRogx8fCT16oxnXx/Xrap\
BRHD/ufHZ08A2IRVw6U6vKDv8TpQH22sAEtUji4/P2AaZmeOxFsYW5FshQr37KBG\
+cBb7rJWLWEJpIXmCpnt37GGCtsACqRegkl7qQ8Q0OiJmtrYLPi00xSstZb+Wv1v\
8B0eTY3POAUXjgl357L5dc6vS99rYFkYeUCTWHaH4d51Z/KgCRYUadboDc2cgNg/\
z2dbO9S3HADegbIsN3fTbjDCruKfvc5ejxlFZ0Xbu6SScQbmkP8t3TPvy/DXGJAh\
NwIDAQAB";

    let pub_key = rsa_load_public_key(public_key_base64).unwrap();
    let value = "test_uuid_value";
    let encrypted = rsa_encrypt(value, &pub_key).unwrap();
    assert!(!encrypted.is_empty());
}
