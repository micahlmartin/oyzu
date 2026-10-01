use crate::builders::task::insert;
use crate::builders::task::unavailable;
use crate::builders::{Builder, Descriptor};
use crate::model::Target;
use anyhow::Result;
use std::path::Path;

pub(super) struct Docker;

impl Builder for Docker {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            ids: &["docker/image"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        ["Dockerfile"]
            .iter()
            .any(|file| path.join(file).is_file())
            .then_some("docker/image")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        target.manager = "docker".into();
        insert(target, "build", &["docker", "build", "."], true);
        unavailable(target, "test", "No image smoke-test contract is configured");

        Ok(())
    }
}
