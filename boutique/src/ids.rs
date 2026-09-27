pub const LEN: usize = 32;

pub fn new() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}
