//! Content verification and publication boundary for the tool installation store.
//! Native handles, rather than unchecked reopened paths, anchor payload access.
mod access;
mod archive;
mod blob;
mod layout;
mod lease;
mod receipt;
mod transaction;
mod tree;

pub(super) use archive::{materialize, materialize_blob};
pub(super) use blob::cache;
pub use blob::VerifiedBlob;
pub(super) use layout::stage;
pub(super) use lease::recover;
pub use lease::LeaseRecovery;
pub(super) use receipt::verify;
pub use receipt::{LeasedToolCommand, ToolArgument, ToolInstallPath, ToolLaunch};
pub(super) use transaction::transact;
pub use transaction::InstallationLease;
pub(super) use tree::inspect;
pub use tree::{TreeEntry, TreeInspection};
