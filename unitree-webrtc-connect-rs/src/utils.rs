use md5::{Digest, Md5};

/// Function to generate MD5 hash of a string
pub fn generate_md5(string: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(string.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// A UUID generation variant matching the JS-like logic from Python util.py
pub fn generate_uuid_js_style() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let template = "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx";
    let mut result = String::with_capacity(36);

    for c in template.chars() {
        if c == 'x' || c == 'y' {
            let r: u8 = rng.gen_range(0..16);
            if c == 'x' {
                result.push_str(&format!("{:x}", r));
            } else {
                result.push_str(&format!("{:x}", (r & 0x3) | 0x8));
            }
        } else {
            result.push(c);
        }
    }
    result
}
