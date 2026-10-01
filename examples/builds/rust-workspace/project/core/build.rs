use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-changed=message.txt");
    let value = fs::read_to_string("message.txt").unwrap();
    let dest = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("message.rs");
    fs::write(
        dest,
        format!("pub const MESSAGE: &str = {:?};", value.trim()),
    )
    .unwrap();
}
