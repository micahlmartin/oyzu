include!(concat!(env!("OUT_DIR"), "/message.rs"));

/// Returns the configured greeting.
///
/// ```
/// assert!(!example_core::greeting().is_empty());
/// ```
pub fn greeting() -> String {
    if cfg!(feature = "shout") {
        MESSAGE.to_uppercase()
    } else {
        MESSAGE.to_owned()
    }
}

pub fn answer() -> String {
    itoa::Buffer::new().format(42).to_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn generated_message() {
        assert_eq!(super::MESSAGE, "Hello, Oyzu!");
        assert_eq!(super::answer(), "42");
        assert!(!super::greeting().is_empty());
    }
}
