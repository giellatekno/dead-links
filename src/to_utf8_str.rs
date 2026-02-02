pub trait ToUtf8Str {
    fn to_utf8_str(&self) -> &str;
}

// just so that I can do `.to_utf8_str()` instead of
// `.to_str().expect("path is valid utf-8")` on every path, and we do assume that all
// paths are valid utf-8
impl ToUtf8Str for std::path::Path {
    fn to_utf8_str(&self) -> &str {
        self.to_str().expect("path is valid utf-8")
    }
}
