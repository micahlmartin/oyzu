use crate::builders::task::insert;
use crate::builders::{Builder, Descriptor};
use crate::model::Target;
use anyhow::Result;
use std::path::Path;

pub(in crate::builders) struct Gradle;

impl Builder for Gradle {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            ids: &["java/gradle"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        ["build.gradle", "build.gradle.kts"]
            .iter()
            .any(|file| path.join(file).is_file())
            .then_some("java/gradle")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        let path = target.path.clone();
        target.manager = "gradle".into();
        let executable = if path.join("gradlew").is_file() {
            if cfg!(windows) {
                "gradlew.bat"
            } else {
                "./gradlew"
            }
        } else {
            "gradle"
        };
        insert(target, "build", &[executable, "--no-daemon", "build"], true);
        insert(target, "test", &[executable, "--no-daemon", "test"], false);

        Ok(())
    }
}
