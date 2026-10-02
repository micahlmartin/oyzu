//! Captured dependency inputs shared by builders and the build engine.
pub(crate) mod images;
pub(crate) mod preparation;
use serde_json::Value;
use std::path::PathBuf;

pub(crate) struct Prepared {
    pub root: PathBuf,
    pub digest: String,
    pub record: Value,
}
