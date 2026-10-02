//! Typed native workspace facts; no scheduling or package execution here.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Metadata {
    schema_version: u32,
    members: Vec<Member>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Member {
    name: String,
    path: String,
    version: String,
    private: bool,
    scripts: BTreeMap<String, String>,
    dependencies: Vec<Edge>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Edge {
    name: String,
    target: String,
    kind: EdgeKind,
    spec: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum EdgeKind {
    Prod,
    Dev,
    Optional,
    Peer,
    PeerOptional,
    Workspace,
}

impl Metadata {
    pub(super) fn read(value: serde_json::Value) -> Result<Option<Self>> {
        if value.is_null() {
            return Ok(None);
        }
        let metadata: Self = serde_json::from_value(value)?;
        if metadata.schema_version != 1
            || metadata.members.is_empty()
            || metadata.members.len() > 1024
        {
            bail!("invalid native npm workspace metadata version or member count");
        }
        let mut names = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for member in &metadata.members {
            if member.name.is_empty()
                || !member
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"@/._-".contains(&b))
                || !names.insert(member.name.as_str())
                || !crate::snapshot::portable(&member.path)
                || member.path.split('/').any(|p| p == "node_modules")
                || !paths.insert(member.path.to_ascii_lowercase())
                || member.version.is_empty()
                || member
                    .version
                    .chars()
                    .any(|c| c.is_whitespace() || c.is_control())
            {
                bail!("invalid or colliding native npm workspace identity");
            }
        }
        for member in &metadata.members {
            let mut edges = BTreeSet::new();
            for edge in &member.dependencies {
                if !names.contains(edge.target.as_str())
                    || edge.name.is_empty()
                    || edge.spec.is_empty()
                    || !edges.insert(edge.name.as_str())
                {
                    bail!("invalid native npm workspace dependency edge");
                }
            }
        }
        Ok(Some(metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn native_workspace_records_reject_escape_and_unknown_dependencies() {
        let value = json!({"schemaVersion":1,"members":[{"name":"@demo/app","path":"packages/app","version":"1.0.0","private":true,"scripts":{"test":"node --test"},"dependencies":[]}]});
        assert!(Metadata::read(value.clone()).unwrap().is_some());
        assert!(Metadata::read(serde_json::Value::Null).unwrap().is_none());
        for path in [
            "../outside",
            "/absolute",
            "packages/../app",
            "node_modules/app",
            "C:/outside",
        ] {
            let mut invalid = value.clone();
            invalid["members"][0]["path"] = json!(path);
            assert!(Metadata::read(invalid).is_err());
        }
        let mut invalid = value.clone();
        invalid["members"][0]["dependencies"] =
            json!([{"name":"missing","target":"missing","kind":"prod","spec":"1.0.0"}]);
        assert!(Metadata::read(invalid).is_err());
        let mut invalid = value.clone();
        let member = value["members"][0].clone();
        invalid["members"].as_array_mut().unwrap().push(member);
        assert!(Metadata::read(invalid).is_err());
    }
}
