//! Package scopes are package ownership, shared by build and development plans.
use super::Metadata;

pub(super) fn exclusions(metadata: &Metadata, path: &str) -> Vec<String> {
    let prefix = if path == "." {
        String::new()
    } else {
        format!("{path}/")
    };
    metadata
        .members
        .iter()
        .filter(|m| m.path != path)
        .filter_map(|m| m.path.strip_prefix(&prefix).map(str::to_owned))
        .collect()
}
