//! Fixed image assembly instructions supplied by a builder, not a project DSL.
//! Builders own runtime/ABI suitability; this module binds copies and launch
//! metadata to captured inputs without shell code, RUN, downloads or secrets.
use super::ImageInput;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Recipe {
    pub base: Base,
    pub copies: Vec<Copy>,
    pub uid: u32,
    pub gid: u32,
    pub workdir: String,
    pub entrypoint: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Base {
    Scratch,
    Captured { reference: String },
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Copy {
    pub source: String,
    pub destination: String,
}

fn path(value: &str, absolute: bool) -> Result<()> {
    if absolute && value == "/" {
        return Ok(());
    }
    let relative = if absolute {
        value.strip_prefix('/').unwrap_or("").trim_end_matches('/')
    } else {
        value
    };
    if value.len() > 1024
        || !crate::snapshot::portable(relative)
        || !relative
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
    {
        bail!("invalid literal image recipe path");
    }
    Ok(())
}

impl Recipe {
    pub fn render(&self, images: &[ImageInput]) -> Result<String> {
        if self.copies.is_empty() || self.copies.len() > 256 {
            bail!("image recipe requires 1 to 256 declared copies");
        }
        path(&self.workdir, true)?;
        if self.entrypoint.is_empty()
            || self.entrypoint.len() > 64
            || self.entrypoint[0].is_empty()
            || self
                .entrypoint
                .iter()
                .any(|arg| arg.len() > 4096 || arg.chars().any(char::is_control))
        {
            bail!("image recipe requires bounded literal entrypoint arguments");
        }
        let base = match &self.base {
            Base::Scratch => {
                if !images.is_empty() {
                    bail!("scratch recipe has unexpected base inputs");
                }
                "scratch"
            }
            Base::Captured { reference } => {
                if images.len() != 1 || images[0].reference != *reference {
                    bail!("image recipe requires its exact captured base binding");
                }
                images[0].validate()?;
                &images[0].name
            }
        };
        let mut result = format!("FROM {base}\nWORKDIR {}\n", self.workdir);
        let mut destinations: Vec<String> = Vec::new();
        for copy in &self.copies {
            path(&copy.source, false)?;
            path(&copy.destination, true)?;
            let destination = copy.destination.trim_end_matches('/').to_ascii_lowercase();
            if destinations.iter().any(|other| {
                other == &destination
                    || destination.starts_with(&format!("{other}/"))
                    || other.starts_with(&format!("{destination}/"))
            }) {
                bail!("overlapping image recipe destinations");
            }
            destinations.push(destination);
            let args = serde_json::to_string(&[&copy.source, &copy.destination])?;
            result.push_str(&format!("COPY --chown={}:{} {args}\n", self.uid, self.gid));
        }
        result.push_str(&format!(
            "USER {}:{}\nENTRYPOINT {}\nCMD []\n",
            self.uid,
            self.gid,
            serde_json::to_string(&self.entrypoint)?
        ));
        if result.len() > 1024 * 1024 {
            bail!("image recipe exceeds 1 MiB");
        }
        Ok(result)
    }

    pub fn digest(&self, images: &[ImageInput]) -> Result<String> {
        Ok(format!(
            "sha256:{:x}",
            Sha256::digest(self.render(images)?.as_bytes())
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Recipe {
        Recipe {
            base: Base::Scratch,
            copies: vec![Copy {
                source: "input/app".into(),
                destination: "/app/app".into(),
            }],
            uid: 65532,
            gid: 65532,
            workdir: "/app".into(),
            entrypoint: vec!["/app/app".into(), "$LITERAL".into()],
        }
    }

    #[test]
    fn generated_definition_is_literal_bounded_and_content_identified() {
        let recipe = fixture();
        let text = recipe.render(&[]).unwrap();
        assert_eq!(text, "FROM scratch\nWORKDIR /app\nCOPY --chown=65532:65532 [\"input/app\",\"/app/app\"]\nUSER 65532:65532\nENTRYPOINT [\"/app/app\",\"$LITERAL\"]\nCMD []\n");
        assert_eq!(
            recipe.digest(&[]).unwrap(),
            format!("sha256:{:x}", Sha256::digest(text))
        );
        let mut changed = recipe.clone();
        changed.uid = 123;
        assert_ne!(recipe.digest(&[]).unwrap(), changed.digest(&[]).unwrap());
        changed.workdir = "/".into();
        assert!(changed.render(&[]).is_ok());
        for invalid in [
            "../escape",
            "input/$VAR",
            "input/*.py",
            "input/a\nRUN evil",
            "https://example.invalid/app",
            "input/[ab]",
            "input/\\escape",
        ] {
            changed = recipe.clone();
            changed.copies[0].source = invalid.into();
            assert!(changed.render(&[]).is_err(), "{invalid}");
        }
        for invalid in ["relative", "/app/../escape", "/app/$VAR", "/app\nRUN evil"] {
            changed = recipe.clone();
            changed.workdir = invalid.into();
            assert!(changed.render(&[]).is_err());
        }
        changed = recipe;
        changed.copies.push(Copy {
            source: "other".into(),
            destination: "/APP/app/child".into(),
        });
        assert!(changed.render(&[]).is_err());
    }

    #[test]
    fn a_registry_name_is_not_a_captured_base() {
        let mut recipe = fixture();
        recipe.base = Base::Captured {
            reference: "python:3.12".into(),
        };
        assert!(recipe.render(&[]).is_err());
        let input = ImageInput {
            reference: "python:3.12".into(),
            name: "docker.io/library/python:3.12".into(),
            store: "images/base-0".into(),
            manifest: format!("sha256:{}", "1".repeat(64)),
            config: format!("sha256:{}", "2".repeat(64)),
            tree_digest: format!("sha256:{}", "3".repeat(64)),
        };
        assert!(recipe
            .render(std::slice::from_ref(&input))
            .unwrap()
            .starts_with("FROM docker.io/library/python:3.12\n"));
        assert!(recipe.render(&[input.clone(), input.clone()]).is_err());
        recipe.base = Base::Scratch;
        assert!(recipe.render(&[input]).is_err());
    }

    #[test]
    fn plan_validation_binds_recipe_bytes_and_excludes_script_fields() {
        let recipe = fixture();
        let mut wire = serde_json::to_value(&recipe).unwrap();
        wire["run"] = serde_json::json!(["curl", "https://example.invalid"]);
        assert!(serde_json::from_value::<Recipe>(wire).is_err());
        let mut mode = super::super::Mode::Buildkit {
            output: "image.oci.tar".into(),
            image_name: "oyzu/test:1".into(),
            context_files: vec!["input/app".into()],
            apparmor_profile: "unconfined".into(),
            dockerfile_digest: recipe.digest(&[]).unwrap(),
            generated_recipe: Some(recipe),
            images: vec![],
        };
        mode.validate().unwrap();
        if let super::super::Mode::Buildkit {
            generated_recipe: Some(recipe),
            ..
        } = &mut mode
        {
            recipe.entrypoint.push("changed".into());
        }
        assert!(mode
            .validate()
            .unwrap_err()
            .to_string()
            .contains("planned Dockerfile identity"));
    }
}
