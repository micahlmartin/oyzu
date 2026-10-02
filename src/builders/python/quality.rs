//! Native quality tasks share one invocation contract for packages and applications.
use crate::{
    builders::{task::insert, BuilderPlan, TaskPlan},
    model::Target,
};

pub(super) fn discover(target: &mut Target) {
    match target.discovery["linter"].selected() {
        "flake8" => insert(target, "lint", &["flake8", "."], true),
        _ => insert(target, "lint", &["ruff", "check", ".", "--no-fix"], true),
    }
    match target.discovery["formatter"].selected() {
        "black" => {
            insert(target, "format-check", &["black", "--check", "."], true);
            insert(target, "format", &["black", "."], false);
        }
        _ => {
            insert(
                target,
                "format-check",
                &["ruff", "format", "--check", "."],
                true,
            );
            insert(target, "format", &["ruff", "format", "."], false);
        }
    }
    target.tasks.get_mut("format").unwrap().mutates_source = true;
}

pub(super) fn plan(target: &Target, plan: &mut BuilderPlan) {
    for task in target.tasks.values() {
        if matches!(task.name.as_str(), "lint" | "format-check" | "format")
            && task
                .argv
                .first()
                .is_some_and(|v| matches!(v.as_str(), "ruff" | "black" | "flake8"))
        {
            let mut argv = vec![
                ".oyzu-build/venv/bin/python".into(),
                "-I".into(),
                "/oyzu/python-quality.py".into(),
            ];
            argv.extend(task.argv.clone());
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
