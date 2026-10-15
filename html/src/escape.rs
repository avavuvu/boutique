// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// !!!!!!!! PLEASE KEEP THIS IN SYNC WITH `maud/src/escape.rs` !!!!!!!!!
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

pub fn escape_to_string(input: &str, output: &mut String) {
    for b in input.bytes() {
        match b {
            b'&' => output.push_str("&amp;"),
            b'<' => output.push_str("&lt;"),
            b'>' => output.push_str("&gt;"),
            b'"' => output.push_str("&quot;"),
            _ => unsafe { output.as_mut_vec().push(b) },
        }
    }
}
