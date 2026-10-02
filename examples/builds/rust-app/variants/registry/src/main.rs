fn greeting(value: u32) -> String {
    format!("Answer: {}", itoa::Buffer::new().format(value))
}

fn main() {
    println!("{}", greeting(42));
}

#[test]
fn formats_registry_dependency_output() {
    assert_eq!(greeting(42), "Answer: 42");
}
