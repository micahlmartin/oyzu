//! Bounded action scheduling over frozen dependencies and private target workspaces.
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Schedule {
    dependencies: Vec<Vec<usize>>,
    targets: Vec<String>,
    jobs: usize,
}
impl Schedule {
    pub fn new(actions: &[Value], jobs: u64) -> Result<Self> {
        if !(1..=65535).contains(&jobs) {
            bail!("invalid execution concurrency");
        }
        let mut ids = BTreeMap::new();
        let mut targets = Vec::new();
        for (index, action) in actions.iter().enumerate() {
            if ids
                .insert(action["id"].as_str().context("missing action id")?, index)
                .is_some()
            {
                bail!("duplicate action id");
            }
            targets.push(
                action["target"]
                    .as_str()
                    .context("missing action target")?
                    .into(),
            );
        }
        let dependencies = actions
            .iter()
            .map(|action| {
                action["dependsOn"]
                    .as_array()
                    .context("missing action dependencies")?
                    .iter()
                    .map(|id| {
                        ids.get(id.as_str().context("invalid action dependency")?)
                            .copied()
                            .context("unknown action dependency")
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let mut remaining: BTreeSet<_> = (0..actions.len()).collect();
        while !remaining.is_empty() {
            let ready: Vec<_> = remaining
                .iter()
                .copied()
                .filter(|&index| dependencies[index].iter().all(|d| !remaining.contains(d)))
                .collect();
            if ready.is_empty() {
                bail!("action dependency cycle");
            }
            for index in ready {
                remaining.remove(&index);
            }
        }
        Ok(Self {
            dependencies,
            targets,
            jobs: jobs as usize,
        })
    }
    pub fn ready(&self, outcomes: &[Value]) -> Result<Vec<usize>> {
        let mut targets = BTreeSet::new();
        let ready: Vec<_> = (0..outcomes.len())
            .filter(|&index| {
                outcomes[index]["status"] == "pending"
                    && self.dependencies[index]
                        .iter()
                        .all(|&dependency| outcomes[dependency]["status"] != "pending")
                    && targets.insert(&self.targets[index])
            })
            .take(self.jobs)
            .collect();
        if ready.is_empty() {
            bail!("no ready action in unfinished plan");
        }
        Ok(ready)
    }
    pub fn permitted(&self, index: usize, outcomes: &[Value]) -> bool {
        self.dependencies[index]
            .iter()
            .all(|&dependency| outcomes[dependency]["status"] == "succeeded")
    }
}

// Workers perform effects; the caller commits results in plan order after all
// workers in the bounded batch finish, before releasing any dependent action.
pub(super) fn parallel<T: Send>(
    indices: &[usize],
    run: impl Fn(usize) -> T + Sync,
) -> Result<Vec<(usize, T)>> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = indices
            .iter()
            .map(|&index| {
                let run = &run;
                (index, scope.spawn(move || run(index)))
            })
            .collect();
        handles
            .into_iter()
            .map(|(index, handle)| {
                handle
                    .join()
                    .map(|value| (index, value))
                    .map_err(|_| anyhow::anyhow!("execution worker panicked"))
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Barrier,
    };
    fn actions() -> Vec<Value> {
        vec![
            json!({"id":"a","target":"a","dependsOn":[]}),
            json!({"id":"b","target":"b","dependsOn":[]}),
            json!({"id":"post","target":"a","dependsOn":["a"]}),
            json!({"id":"consumer","target":"c","dependsOn":["post"]}),
        ]
    }
    #[test]
    fn jobs_dependencies_and_failed_hook_boundaries_control_admission() {
        let actions = actions();
        let schedule = Schedule::new(&actions, 2).unwrap();
        let mut outcomes = vec![json!({"status":"pending"}); 4];
        assert_eq!(schedule.ready(&outcomes).unwrap(), vec![0, 1]);
        assert_eq!(
            Schedule::new(&actions, 1)
                .unwrap()
                .ready(&outcomes)
                .unwrap(),
            vec![0]
        );
        outcomes[0]["status"] = json!("failed");
        outcomes[1]["status"] = json!("succeeded");
        assert_eq!(schedule.ready(&outcomes).unwrap(), vec![2]);
        assert!(!schedule.permitted(2, &outcomes));
        outcomes[2]["status"] = json!("blocked");
        assert_eq!(schedule.ready(&outcomes).unwrap(), vec![3]);
        assert!(!schedule.permitted(3, &outcomes));
        for jobs in [0, 65536] {
            assert!(Schedule::new(&actions, jobs).is_err());
        }
        let mut cyclic = actions.clone();
        cyclic[0]["dependsOn"] = json!(["post"]);
        assert!(Schedule::new(&cyclic, 2).is_err());
        cyclic[0]["dependsOn"] = json!(["missing"]);
        assert!(Schedule::new(&cyclic, 2).is_err());
    }
    #[test]
    fn workers_overlap_and_return_in_plan_order() {
        let barrier = Barrier::new(2);
        let active = AtomicUsize::new(0);
        let results = parallel(&[0, 1], |index| {
            active.fetch_add(1, Ordering::SeqCst);
            barrier.wait();
            assert_eq!(active.load(Ordering::SeqCst), 2);
            index + 10
        })
        .unwrap();
        assert_eq!(results, vec![(0, 10), (1, 11)]);
    }
}
