use lz4_flex::block::compress;
use unitree_webrtc_core_rs::lidar::native::decode_native_core;

#[test]
fn test_native_decoder_core() {
    // Uncompressed mock payload (e.g. 0x800 * 2 length bytes initialized to 0)
    let mut uncompressed = vec![0u8; 0x800 * 2];
    // Add one mock bit (x=0, y=0, z=0)
    uncompressed[0] = 0b1000_0000;

    let compressed = compress(&uncompressed);

    let origin = [1.0, 2.0, 3.0];
    let resolution = 0.05;

    let result = decode_native_core(&compressed, uncompressed.len(), origin, resolution);
    assert!(result.is_ok());

    let points = result.unwrap();
    assert_eq!(points.len(), 1);

    // x=0*res + org[0] = 1.0
    // y=0*res + org[1] = 2.0
    // z=0*res + org[2] = 3.0
    let p = points[0];
    assert!((p[0] - 1.0).abs() < f64::EPSILON);
    assert!((p[1] - 2.0).abs() < f64::EPSILON);
    assert!((p[2] - 3.0).abs() < f64::EPSILON);
}

#[test]
fn test_native_decoder_core_invalid_lz4() {
    let compressed: Vec<u8> = vec![1, 2, 3, 4, 5];
    let origin = [0.0, 0.0, 0.0];
    let resolution = 0.05;

    let result = decode_native_core(&compressed, 100, origin, resolution);
    assert!(result.is_err());
}
