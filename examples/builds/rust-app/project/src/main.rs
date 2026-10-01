fn greeting(name: &str) -> String { format!("Hello, {name}!") }
fn main() { println!("{}", greeting("Oyzu")); }
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn greets() { assert_eq!(greeting("Oyzu"), "Hello, Oyzu!"); }
}
