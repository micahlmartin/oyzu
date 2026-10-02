//! Bounded static chart-archive observations. Never extracts or executes inputs.
use anyhow::{bail, Context, Result};
use std::{collections::BTreeSet, io::Read};

#[derive(Default)]
pub(super) struct Budget {
    total: usize,
    count: usize,
}

pub(super) fn suites(bytes: &[u8], budget: &mut Budget) -> Result<Vec<String>> {
    fn visit(
        bytes: &[u8],
        prefix: &str,
        depth: usize,
        total: &mut usize,
        count: &mut usize,
        suites: &mut Vec<String>,
    ) -> Result<()> {
        if depth > 16 {
            bail!("Helm archive depth exceeds 16");
        }
        let mut decoded = Vec::new();
        flate2::read::MultiGzDecoder::new(bytes)
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut decoded)?;
        *total += decoded.len();
        if *total > 64 * 1024 * 1024 {
            bail!("Helm expanded archive inputs exceed 64 MiB");
        }
        let mut names = BTreeSet::new();
        let mut roots = BTreeSet::new();
        let mut metadata = false;
        for entry in tar::Archive::new(decoded.as_slice()).entries()? {
            let entry = entry?;
            *count += 1;
            if *count > 4096 {
                bail!("Helm archive entry count exceeds 4096");
            }
            let path = entry
                .path()?
                .to_str()
                .context("non-UTF8 Helm archive path")?
                .to_string();
            if !entry.header().entry_type().is_file()
                || !crate::snapshot::portable(&path)
                || !names.insert(path.to_lowercase())
            {
                bail!("unsafe or duplicate Helm archive entry: {path}");
            }
            let parts: Vec<_> = path.split('/').collect();
            if parts.len() < 2 {
                bail!("Helm archive requires one chart root");
            }
            roots.insert(parts[0].to_string());
            if parts.len() == 2 && parts[1] == "Chart.yaml" {
                metadata = true;
            }
            // Native unpacked chart ancestry is root/(charts/name)*/tests/file.
            let rest = &parts[1..];
            let mut offset = 0;
            while offset + 2 < rest.len()
                && rest[offset] == "charts"
                && !rest[offset + 1].starts_with(['.', '_'])
            {
                offset += 2;
            }
            if rest.len() == offset + 2
                && rest[offset] == "tests"
                && rest[offset + 1].ends_with("_test.yaml")
            {
                suites.push(format!("{prefix}{path}"));
                if suites.len() > 128 {
                    bail!("Helm native suite count exceeds 128");
                }
            }
            if rest.len() == offset + 2
                && rest[offset] == "charts"
                && rest[offset + 1].ends_with(".tgz")
                && !rest[offset + 1].starts_with(['.', '_'])
            {
                let mut nested = Vec::new();
                entry.take(4 * 1024 * 1024 + 1).read_to_end(&mut nested)?;
                if nested.len() > 4 * 1024 * 1024 {
                    bail!("Helm nested archive exceeds 4 MiB");
                }
                visit(
                    &nested,
                    &format!("{prefix}{path}!/"),
                    depth + 1,
                    total,
                    count,
                    suites,
                )?;
            }
        }
        if roots.len() != 1 || !metadata {
            bail!("Helm archive requires one root with Chart.yaml");
        }
        Ok(())
    }
    let mut found = Vec::new();
    visit(
        bytes,
        "",
        0,
        &mut budget.total,
        &mut budget.count,
        &mut found,
    )?;
    found.sort();
    Ok(found)
}
