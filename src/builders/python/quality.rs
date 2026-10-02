//! Native Ruff tasks share one invocation contract for packages and applications.
use crate::{
    builders::{task::insert, BuilderPlan, TaskPlan},
    model::Target,
};

pub(super) fn discover(target: &mut Target) {
    insert(target, "lint", &["ruff", "check", "."], true);
    insert(
        target,
        "format-check",
        &["ruff", "format", "--check", "."],
        true,
    );
    insert(target, "format", &["ruff", "format", "."], false);
    target.tasks.get_mut("format").unwrap().mutates_source = true;
}

pub(super) fn plan(target: &Target, plan: &mut BuilderPlan) {
    for task in target.tasks.values() {
        if task.argv.first().is_some_and(|v| v == "ruff") {
            let mut argv = task.argv.clone();
            argv[0] = ".oyzu-build/venv/bin/ruff".into();
            if task.name == "lint" {
                argv.push("--no-fix".into());
            }
            plan.tasks.insert(
                task.name.clone(),
                TaskPlan {
                    argv,
                    ..TaskPlan::default()
                },
            );
        }
    }
}
