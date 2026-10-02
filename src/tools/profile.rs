//! Explicit shell profile block installation/removal. Owns only the bounded
//! profile edit; activation and session state remain with the shell subsystem.
use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path};

const BEGIN: &str = "\n# >>> oyzu shell profile v1\n";
const END: &str = "# <<< oyzu shell profile v1\n";

fn hash(body: &str) -> String {
    format!("{:x}", Sha256::digest(body.as_bytes()))
}

fn block(shell: &str) -> Result<String> {
    let frontend = std::env::current_exe()?;
    let frontend = frontend.to_str().context("frontend path must be UTF-8")?;
    let body = match shell {
        "bash" | "zsh" => {
            let quoted = frontend.replace('\'', "'\\''");
            format!("eval \"$('{}' activate {})\"\n", quoted, shell)
        }
        "pwsh" => format!(
            "(& '{}' activate pwsh) | Out-String | Invoke-Expression\n",
            frontend.replace('\'', "''")
        ),
        _ => anyhow::bail!("supported shells are bash, zsh and pwsh"),
    };
    Ok(format!(
        "{BEGIN}# shell: {shell}\n# sha256: {}\n{body}{END}",
        hash(&body)
    ))
}

/// Edit only our recognized, unmodified block in an explicitly chosen UTF-8
/// profile. Preserve surrounding bytes and permissions; never execute the file.
pub fn edit(path: &Path, shell: &str, install: bool) -> Result<bool> {
    ensure!(
        matches!(shell, "bash" | "zsh" | "pwsh"),
        "unsupported shell"
    );
    let path = std::path::absolute(path)?;
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(value) => {
            ensure!(
                value.is_file() && !value.file_type().is_symlink(),
                "profile must be a regular file; specify the real target for a symlink"
            );
            Some(value)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let original = if metadata.is_some() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };
    let starts: Vec<_> = original
        .match_indices(BEGIN)
        .map(|(index, _)| index)
        .collect();
    let ends: Vec<_> = original
        .match_indices(END)
        .map(|(index, _)| index)
        .collect();
    ensure!(
        starts.len() == ends.len() && starts.len() <= 1,
        "profile has incomplete or duplicate Oyzu blocks; reconcile manually"
    );
    let replacement = if install {
        block(shell)?
    } else {
        String::new()
    };
    let updated = if let (Some(start), Some(end)) = (starts.first(), ends.first()) {
        ensure!(*end > *start, "invalid Oyzu profile block");
        let contents = &original[start + BEGIN.len()..*end];
        let (owner, contents) = contents
            .split_once('\n')
            .context("invalid profile block owner")?;
        ensure!(
            owner == format!("# shell: {shell}"),
            "profile block belongs to another shell"
        );
        let (digest, body) = contents
            .split_once('\n')
            .context("invalid profile block digest")?;
        ensure!(
            digest == format!("# sha256: {}", hash(body)),
            "Oyzu profile block was edited; preserve or reconcile those changes manually"
        );
        format!(
            "{}{}{}",
            &original[..*start],
            replacement,
            &original[end + END.len()..]
        )
    } else {
        ensure!(
            !original.contains("# >>> oyzu shell profile")
                && !original.contains("# <<< oyzu shell profile"),
            "unrecognized Oyzu profile marker; reconcile manually"
        );
        format!("{original}{replacement}")
    };
    if updated == original {
        return Ok(false);
    }
    let parent = path.parent().context("profile has no parent")?;
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(updated.as_bytes())?;
    if let Some(metadata) = metadata {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())?;
    }
    temporary.as_file().sync_all()?;
    let current = std::fs::read_to_string(&path).or_else(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Ok(String::new())
        } else {
            Err(error)
        }
    })?;
    ensure!(current == original, "profile changed during edit; retry");
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(true)
}
