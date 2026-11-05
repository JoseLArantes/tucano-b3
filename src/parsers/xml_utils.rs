pub fn local_name(tag: &[u8]) -> String {
    let name_str = String::from_utf8_lossy(tag);
    if let Some(pos) = name_str.rfind(':') {
        name_str[pos + 1..].to_string()
    } else {
        name_str.to_string()
    }
}
