//! Shared native Java JUnit suite composition; result admission remains engine-owned.
pub(super) const RUNTIME: crate::builders::RuntimeFile = crate::builders::RuntimeFile {
    name: "java-junit.py",
    contents: include_str!("runtime/junit.py"),
};
