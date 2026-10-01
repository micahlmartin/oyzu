include!(concat!(env!("OUT_DIR"), "/message.rs"));
pub fn greeting() -> String {
 if cfg!(feature = "shout") { MESSAGE.to_uppercase() } else { MESSAGE.to_owned() }
}
#[cfg(test)] mod tests { #[test] fn generated_message() { assert_eq!(super::MESSAGE, "Hello, Oyzu!"); } }
