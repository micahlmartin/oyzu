//! Compile-only probe of the existing upstream API, without invented methods.
fn main() {
    let _future = mise::config::Config::load_from_config_files(Default::default(), false);
}
