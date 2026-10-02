//! Admission of literal image references before handing them to native tooling.
//! Native resolution still owns full reference parsing and content identity.
pub(crate) fn literal_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && !value.contains("://")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._:@-".contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_references_reject_command_syntax_urls_and_unbounded_input() {
        for value in [
            "alpine:3.22",
            "registry.example:5000/team/image:v1",
            &format!("registry.example/image@sha256:{}", "a".repeat(64)),
        ] {
            assert!(literal_reference(value));
        }
        for value in [
            "",
            "--help",
            "https://user:password@example/image",
            "image\nnext",
            "$(download)",
            &"x".repeat(513),
        ] {
            assert!(!literal_reference(value));
        }
    }
}
