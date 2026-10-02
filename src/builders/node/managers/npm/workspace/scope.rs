//! Package scopes are shared across native Node managers.
use super::Metadata;

pub(super) fn exclusions(metadata: &Metadata, path: &str) -> Vec<String> {
    crate::builders::node::workspace::exclusions(
        metadata.members.iter().map(|m| m.path.as_str()),
        path,
    )
}
