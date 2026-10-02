//! Read raw local Git objects without checkout, filters, index writes or network.
use crate::snapshot;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

#[derive(PartialEq, Eq)]
pub(super) struct File {
    pub digest: String,
    pub executable: bool,
}

pub(super) fn baseline(root: &Path, reference: &str) -> Result<(String, BTreeMap<String, File>)> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{reference}^{{commit}}"),
        ])
        .output()?;
    if !output.status.success() {
        bail!("local baseline commit is unavailable");
    }
    let commit = String::from_utf8(output.stdout)?.trim().to_owned();
    if !object_id(&commit) {
        bail!("invalid resolved Git commit");
    }
    let prefix = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--show-prefix"])
        .output()?;
    if !prefix.status.success() {
        bail!("Git workspace prefix is unavailable");
    }
    let prefix = String::from_utf8(prefix.stdout)?;
    let tree = if prefix.trim().is_empty() {
        commit.clone()
    } else {
        format!("{commit}:{}", prefix.trim().trim_end_matches('/'))
    };
    let listing = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-tree", "--full-tree", "-r", "-z", &tree])
        .output()?;
    if !listing.status.success() {
        bail!("local baseline tree is unavailable");
    }
    let mut entries = Vec::new();
    for record in listing.stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let record = std::str::from_utf8(record)?;
        let (metadata, path) = record.split_once('\t').context("invalid Git tree record")?;
        if !snapshot::source_path_included(path) {
            continue;
        }
        let fields: Vec<_> = metadata.split_whitespace().collect();
        if !snapshot::portable(path)
            || fields.len() != 3
            || fields[1] != "blob"
            || !matches!(fields[0], "100644" | "100755")
            || !object_id(fields[2])
        {
            bail!("unsupported baseline input type");
        }
        if entries.len() >= 100_000 {
            bail!("baseline exceeds source entry limit");
        }
        entries.push((path.to_owned(), fields[2].to_owned(), fields[0] == "100755"));
    }
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut input = child.stdin.take().context("missing Git object input")?;
    let mut output = BufReader::new(child.stdout.take().context("missing Git object output")?);
    let result = (|| -> Result<_> {
        let mut files = BTreeMap::new();
        let mut total = 0u64;
        for (path, object, executable) in entries {
            writeln!(input, "{object}")?;
            input.flush()?;
            let mut header = String::new();
            output.read_line(&mut header)?;
            let fields: Vec<_> = header.split_whitespace().collect();
            if fields.len() != 3 || fields[0] != object || fields[1] != "blob" {
                bail!("missing baseline blob");
            }
            let size: u64 = fields[2].parse()?;
            total = total.checked_add(size).context("baseline size overflow")?;
            if total > 10 * 1024 * 1024 * 1024 {
                bail!("baseline exceeds source byte limit");
            }
            let mut hash = Sha256::new();
            let mut remaining = size;
            let mut buffer = [0u8; 65536];
            while remaining > 0 {
                let count = remaining.min(buffer.len() as u64) as usize;
                output.read_exact(&mut buffer[..count])?;
                hash.update(&buffer[..count]);
                remaining -= count as u64;
            }
            let mut newline = [0];
            output.read_exact(&mut newline)?;
            if newline != [b'\n'] {
                bail!("invalid Git blob delimiter");
            }
            files.insert(
                path,
                File {
                    digest: format!("sha256:{:x}", hash.finalize()),
                    executable,
                },
            );
        }
        Ok(files)
    })();
    drop(input);
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait()?;
    if !status.success() {
        bail!("baseline object read failed");
    }
    Ok((commit, result?))
}
fn object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}
