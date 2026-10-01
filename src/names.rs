//! Identifier rules shared by configuration and generated artifact contracts.
use sha2::{Digest, Sha256};

pub(crate) fn valid(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

/// Preserve readable native names, using a stable suffix when normalization or
/// shortening is required. The original native name remains in the filename.
pub(crate) fn scoped(prefix: &str, native: &str) -> String {
    let original = format!("{prefix}-{native}");
    if valid(&original) {
        return original;
    }
    let mut normalized: String = original
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    normalized.truncate(51);
    format!(
        "{normalized}-{}",
        &format!("{:x}", Sha256::digest(original.as_bytes()))[..12]
    )
}
