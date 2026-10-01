use crate::model::{Target, Task};
use std::collections::BTreeMap;

pub(super) fn task(target: &Target, name: &str, argv: &[&str], stage: bool) -> Task {
    Task {
        name: name.into(),
        target: target.name.clone(),
        provider: target.manager.clone(),
        argv: argv.iter().map(|s| s.to_string()).collect(),
        cwd: target.path.clone(),
        env: BTreeMap::new(),
        depends_on: vec![],
        availability: None,
        build_stage: stage,
        mutates_source: false,
        stdout_must_be_empty: false,
    }
}

pub(super) fn insert(target: &mut Target, name: &str, argv: &[&str], stage: bool) {
    let value = task(target, name, argv, stage);
    target.tasks.insert(name.into(), value);
}

pub(crate) fn unavailable(target: &mut Target, name: &str, reason: &str) {
    let mut value = task(target, name, &[], false);
    value.availability = Some(reason.into());
    target.tasks.entry(name.into()).or_insert(value);
}
