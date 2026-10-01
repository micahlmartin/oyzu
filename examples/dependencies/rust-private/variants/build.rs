fn main() { assert!(std::env::var("CARGO_REGISTRIES_CORPORATE_TOKEN").is_err(), "upstream token exposed to build script"); }
