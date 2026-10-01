include!(concat!(env!("OUT_DIR"), "/message.rs"));
pub fn greeting() -> String {
    if cfg!(feature = "shout") {
        MESSAGE.to_uppercase()
    } else {
        MESSAGE.to_owned()
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn generated_message() {
        assert_eq!(super::MESSAGE, "Hello, Oyzu!");
        let expected = if cfg!(feature = "shout") {
            super::MESSAGE.to_uppercase()
        } else {
            super::MESSAGE.to_owned()
        };
        assert_eq!(super::greeting(), expected);
    }
}
