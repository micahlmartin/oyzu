//! Inventory parser guard: reject YAML graph features before deserialization.
use anyhow::{bail, Result};
use std::collections::BTreeSet;
use yaml_rust2::parser::{Event, Parser};
enum Frame {
    Map { key: bool, seen: BTreeSet<String> },
    Sequence,
}
pub fn validate(text: &str) -> Result<()> {
    let mut parser = Parser::new_from_str(text);
    let mut stack = Vec::new();
    let mut entries = 0;
    let mut documents = 0;
    loop {
        let (event, _) = parser
            .next_token()
            .map_err(|_| anyhow::anyhow!("CONFIG_SYNTAX: invalid build.yaml"))?;
        entries += 1;
        if entries > 20000 || stack.len() > 32 {
            bail!("CONFIG_LIMIT: YAML entry or depth limit");
        }
        match event {
            Event::StreamEnd => break,
            Event::DocumentStart => {
                documents += 1;
                if documents > 1 {
                    bail!("CONFIG_SYNTAX: one YAML document required");
                }
            }
            Event::Alias(_) => bail!("CONFIG_SYNTAX: YAML aliases are forbidden"),
            Event::Scalar(value, _, _, tag) => {
                if tag.is_some() {
                    bail!("CONFIG_SYNTAX: YAML tags are forbidden");
                }
                if let Some(Frame::Map { key, seen }) = stack.last_mut() {
                    if *key {
                        if value == "<<" {
                            bail!("CONFIG_SYNTAX: YAML merge keys are forbidden");
                        }
                        if !seen.insert(value) {
                            bail!("CONFIG_DUPLICATE: duplicate YAML key");
                        }
                    }
                    *key = !*key;
                }
            }
            Event::MappingStart(_, ref tag) | Event::SequenceStart(_, ref tag) => {
                if tag.is_some() {
                    bail!("CONFIG_SYNTAX: YAML tags are forbidden");
                }
                if let Some(Frame::Map { key, .. }) = stack.last_mut() {
                    if *key {
                        bail!("CONFIG_SYNTAX: YAML mapping keys must be strings");
                    }
                    *key = true;
                }
                if matches!(event, Event::MappingStart(..)) {
                    stack.push(Frame::Map {
                        key: true,
                        seen: BTreeSet::new(),
                    });
                } else {
                    stack.push(Frame::Sequence);
                }
            }
            Event::MappingEnd | Event::SequenceEnd => {
                stack.pop();
            }
            _ => {}
        }
    }
    Ok(())
}
