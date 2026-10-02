//! Provision physical fixture roots for the store's no-follow contract.
pub fn directory() -> std::io::Result<tempfile::TempDir> {
    // macOS commonly exposes its temporary root through /var, a symlink to
    // /private/var. Resolve only this trusted test parent, before creating any
    // fixture files or hostile links. Never canonicalize an input under test.
    tempfile::tempdir_in(std::env::temp_dir().canonicalize()?)
}
