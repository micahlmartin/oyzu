//! Composition of captured configuration and non-executing target selection.
use crate::{
    config::{
        operations::{self, Command},
        session::{Options, Session},
    },
    discovery,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::path::Path;
pub fn configuration(command: &Command, directory: &Path, options: &Options) -> Result<Value> {
    match configure(command, directory, options) {
        Err(error) if matches!(command, Command::Show | Command::Explain { .. }) => {
            // Inspection must work even when policy or ordinary syntax blocks
            // execution. Never serialize unreviewed parser/transport error bodies.
            let description = format!("{error:#}");
            let code = description
                .split(|c: char| c == ':' || c.is_whitespace())
                .find(|part| part.starts_with("CONFIG_") || part.starts_with("POLICY_"))
                .unwrap_or("CONFIG_UNRESOLVED");
            Ok(json!({"status":"unresolved","values":null,"diagnostics":[{
                "code":code,"severity":"error","key":null,"sourceSpan":null,"target":null,
                "message":"Configuration resolution is blocked; no effective snapshot is available",
                "remedy":"Run config validate for local diagnostics, or config status and config refresh for managed policy"
            }]}))
        }
        result => result,
    }
}
fn configure(command: &Command, directory: &Path, options: &Options) -> Result<Value> {
    if matches!(
        command,
        Command::Status | Command::Refresh | Command::Set { .. } | Command::Unset { .. }
    ) {
        return operations::execute(command, directory, options);
    }
    // Protected enrollment and source capture precede target discovery.
    let session = Session::open(directory, options)?;
    let inventory = discovery::inventory::select(&session.root)?;
    let mut selected = vec![directory.canonicalize()?];
    selected.extend(inventory.targets.into_iter().map(|target| target.path));
    selected.sort();
    selected.dedup();
    operations::inspect(command, directory, session, selected, inventory.diagnostics)
}
