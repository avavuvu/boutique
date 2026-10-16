use argon2::password_hash::rand_core::{OsRng, RngCore};

use crate::hex;

pub const LEN: usize = 32;

pub fn new() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    OsRng.fill_bytes(&mut buffer);
    hex::encode(&buffer)
}
