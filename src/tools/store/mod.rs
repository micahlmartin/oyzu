//! Content verification and publication boundary for the tool installation store.
//! Native handles, rather than unchecked reopened paths, anchor payload access.
mod access;
mod tree;

pub(super) use tree::inspect;
pub use tree::{TreeEntry, TreeInspection};
