use oyzu::config::{
    constraints::{Constraints, Entry},
    registry::{Registry, Scope},
    resolve::{resolve, Selection},
    sources::ConfigSource,
};
use serde_json::json;
use std::path::Path;
fn source(id: &str, scope: Scope, text: &str) -> ConfigSource {
    ConfigSource::parse(id, Path::new("."), scope, false, text, &Registry::default()).unwrap()
}
#[test]
fn cascade_profiles_and_atomic_tasks() {
    let sources=vec![source("user",Scope::User,"[build]\njobs=4\n[profiles.dev.build]\njobs=8\n[tasks.hello]\nargv=['old']\n[tasks.hello.env]\nOLD='yes'\n"),source("project",Scope::Project,"[profile]\ndefault='dev'\n[build]\njobs=6\n[profiles.dev.build]\njobs=12\n[tasks.hello]\nargv=['new']\n"),source("local",Scope::Local,"[build]\njobs=10\n")];
    let result = resolve(
        &sources,
        &Registry::default(),
        false,
        &Selection::default(),
        Constraints::default(),
        false,
    )
    .unwrap();
    assert_eq!(result.get("build.jobs"), Some(&json!(10)));
    assert_eq!(result.profile.as_deref(), Some("dev"));
    assert!(result.get("tasks.hello").unwrap().get("env").is_none());
    assert_eq!(result.origins["build.jobs"].len(), 6);
}
#[test]
fn optional_compatibility_and_inactive_requirements() {
    let s = source(
        "project",
        Scope::Project,
        "[future]\nfoo=42\n[profiles.future.compatibility]\nrequires=['future/v99']\n",
    );
    let result = resolve(
        std::slice::from_ref(&s),
        &Registry::default(),
        false,
        &Selection::default(),
        Constraints::default(),
        false,
    )
    .unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert!(resolve(
        &[s],
        &Registry::default(),
        false,
        &Selection::default(),
        Constraints::default(),
        true
    )
    .is_err());
    assert!(ConfigSource::parse(
        "bad",
        Path::new("."),
        Scope::Project,
        false,
        "[build]\njobs='secret'",
        &Registry::default()
    )
    .unwrap_err()
    .to_string()
    .contains("CONFIG_INVALID_VALUE"));
}
#[test]
fn removal_and_reintroduction() {
    let a = source("a", Scope::Project, "[env]\nHELLO='one'\n");
    let b = source("b", Scope::Local, "[overrides]\nremove=['env.HELLO']\n");
    let result = resolve(
        &[a.clone(), b.clone()],
        &Registry::default(),
        false,
        &Selection::default(),
        Constraints::default(),
        false,
    )
    .unwrap();
    assert!(result.get("env.HELLO").is_none());
    assert!(result.removed.contains("env.HELLO"));
    let c = source("c", Scope::Invocation, "[env]\nHELLO='two'\n");
    let result = resolve(
        &[a, b, c],
        &Registry::default(),
        false,
        &Selection::default(),
        Constraints::default(),
        false,
    )
    .unwrap();
    assert_eq!(result.get("env.HELLO"), Some(&json!("two")));
    assert!(result.removed.is_empty());
}
#[test]
fn restrictive_constraints_and_required_union() {
    let registry = Registry::default();
    let mut constraints = Constraints::default();
    constraints
        .add(
            "corp",
            "build.jobs",
            Entry {
                maximum: Some(16),
                ..Default::default()
            },
            &registry,
        )
        .unwrap();
    constraints
        .add(
            "local",
            "build.jobs",
            Entry {
                maximum: Some(8),
                ..Default::default()
            },
            &registry,
        )
        .unwrap();
    constraints
        .add(
            "corp",
            "checks.required",
            Entry {
                required: Some(vec![json!("tests")]),
                ..Default::default()
            },
            &registry,
        )
        .unwrap();
    let mut values = std::collections::BTreeMap::from([
        ("build.jobs".into(), json!(8)),
        ("checks.required".into(), json!([])),
    ]);
    constraints.apply(&mut values, &Default::default()).unwrap();
    assert_eq!(values["checks.required"], json!(["tests"]));
    values.insert("build.jobs".into(), json!(9));
    assert!(constraints.apply(&mut values, &Default::default()).is_err());
    assert!(constraints
        .add(
            "bad",
            "build.jobs",
            Entry {
                minimum: Some(10),
                ..Default::default()
            },
            &registry
        )
        .is_err());
}
#[test]
fn redaction_and_identity() {
    let registry = Registry::default();
    let a = source(
        "one",
        Scope::Project,
        "[env]\nTOKEN='never print this'\n[ui]\ncolor='always'\n",
    );
    let b = source(
        "two",
        Scope::Project,
        "[env]\nTOKEN='never print this'\n[ui]\ncolor='never'\n",
    );
    let first = resolve(
        &[a],
        &registry,
        false,
        &Selection::default(),
        Constraints::default(),
        false,
    )
    .unwrap();
    let second = resolve(
        &[b],
        &registry,
        false,
        &Selection::default(),
        Constraints::default(),
        false,
    )
    .unwrap();
    assert_eq!(first.digest, second.digest);
    assert!(!first
        .explain(&registry)
        .to_string()
        .contains("never print this"));
}
#[test]
fn ci_excludes_personal_computation() {
    let a=source("user",Scope::User,"[env]\nPRIVATE='secret'\n[ui]\ncolor='never'\n[profile]\ndefault='dev'\n[profiles.dev.build]\njobs=1\n");
    let result = resolve(
        &[a],
        &Registry::default(),
        true,
        &Selection::default(),
        Constraints::default(),
        false,
    )
    .unwrap();
    assert!(result.get("env.PRIVATE").is_none());
    assert_eq!(result.get("ui.color"), Some(&json!("never")));
    assert_eq!(result.profile.as_deref(), Some("ci"));
}
#[test]
fn strict_json_rejects_duplicates_at_any_depth() {
    assert!(oyzu::config::policy::strict_json(br#"{"a":{"b":1,"b":2}}"#).is_err());
}
#[test]
fn edit_preserves_unknown_comments_and_detects_races() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("oyzu.toml");
    std::fs::write(
        &path,
        "# greeting\n[build]\njobs = 4 # capacity\n[future]\nthing = 'retained'\n",
    )
    .unwrap();
    let registry = Registry::default();
    let mut edit = oyzu::config::edit::Edit::read(&path).unwrap();
    edit.change("build.jobs", Some(json!(8)), None, &registry)
        .unwrap();
    edit.commit(&path, Scope::Project, false, &registry)
        .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("# greeting"));
    assert!(text.contains("# capacity"));
    assert!(text.contains("thing = 'retained'"));
    let edit = oyzu::config::edit::Edit::read(&path).unwrap();
    std::fs::write(&path, "# concurrent edit").unwrap();
    assert!(edit
        .commit(&path, Scope::Project, false, &registry)
        .unwrap_err()
        .to_string()
        .contains("CONFIG_EDIT_CONFLICT"));
}

#[test]
fn arrays_replace_and_policy_locks_conflict_without_last_writer_wins() {
    let registry = Registry::default();
    let first = source(
        "first",
        Scope::Project,
        "[checks]\nrequired=['tests','coverage']\n",
    );
    let second = source("second", Scope::Local, "[checks]\nrequired=[]\n");
    let result = resolve(
        &[first, second],
        &registry,
        false,
        &Selection::default(),
        Constraints::default(),
        false,
    )
    .unwrap();
    assert_eq!(result.get("checks.required"), Some(&json!([])));
    let mut constraints = Constraints::default();
    constraints
        .add(
            "one",
            "cache.write",
            Entry {
                locked: Some(true),
                value: Some(json!(false)),
                ..Default::default()
            },
            &registry,
        )
        .unwrap();
    assert!(constraints
        .add(
            "two",
            "cache.write",
            Entry {
                locked: Some(true),
                value: Some(json!(true)),
                ..Default::default()
            },
            &registry
        )
        .is_err());
}
#[test]
fn configuration_capture_is_immutable() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("oyzu.toml");
    std::fs::write(&path, "[build]\njobs=3\n").unwrap();
    let session = oyzu::config::session::Session::open(
        root.path(),
        &oyzu::config::session::Options {
            root: Some(root.path().into()),
            ..Default::default()
        },
    )
    .unwrap();
    std::fs::write(&path, "[build]\njobs=9\n").unwrap();
    assert_eq!(
        session
            .resolve(root.path(), false)
            .unwrap()
            .get("build.jobs"),
        Some(&json!(3))
    );
}
#[test]
fn yaml_rejects_aliases_duplicates_merge_keys_and_tags() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("build.yaml");
    for text in [
        "a: {uses: node/package}\na: {uses: go/app}\n",
        "a: &base {uses: node/package}\nb: *base\n",
        "a: {uses: !custom node/package}\n",
        "a: {uses: node/package, <<: {path: .}}\n",
    ] {
        std::fs::write(&path, text).unwrap();
        assert!(oyzu::config::targets(root.path()).is_err(), "{text}");
    }
}
#[test]
fn edits_support_inline_tables_and_keep_optional_fields() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("oyzu.toml");
    std::fs::write(&path, "build = { jobs = 3, future = true } # keep\n").unwrap();
    let registry = Registry::default();
    let mut edit = oyzu::config::edit::Edit::read(&path).unwrap();
    edit.change("build.jobs", Some(json!(8)), None, &registry)
        .unwrap();
    edit.commit(&path, Scope::Project, false, &registry)
        .unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("future = true"));
    assert!(text.contains("# keep"));
}
#[test]
fn optional_task_fields_warn_but_cannot_change_the_typed_task() {
    let source = source(
        "project",
        Scope::Project,
        "[tasks.example]\nargv=['echo','ok']\nfuture = 'inert'\n",
    );
    assert_eq!(source.diagnostics.len(), 1);
    assert!(source.base.values["tasks.example"].get("future").is_none());
}
#[test]
fn apparmor_legacy_setting_uses_the_same_constraint_registry() {
    let registry = Registry::default();
    assert!(registry.definition("docker.apparmorProfile").is_some());
    let mut constraints = Constraints::default();
    constraints
        .add(
            "admin",
            "docker.apparmorProfile",
            Entry {
                locked: Some(true),
                value: Some(json!("required-profile")),
                ..Default::default()
            },
            &registry,
        )
        .unwrap();
    let requested = source(
        "legacy-env",
        Scope::Invocation,
        "[docker]\napparmorProfile='unconfined'\n",
    );
    assert!(resolve(
        &[requested],
        &registry,
        false,
        &Selection::default(),
        constraints,
        false
    )
    .is_err());
}

#[test]
fn optional_diagnostics_locate_base_and_profile_values_without_exporting_them() {
    let text =
        "[build]\nfuture='sensitive-base'\n[profiles.dev.build]\nfuture='sensitive-profile'\n";
    let parsed = source("project", Scope::Project, text);
    assert_eq!(parsed.diagnostics.len(), 2);
    for (diagnostic, expected) in parsed
        .diagnostics
        .iter()
        .zip(["'sensitive-base'", "'sensitive-profile'"])
    {
        let (start, end) = diagnostic.span.unwrap();
        assert_eq!(&text[start..end], expected);
        let public = serde_json::to_value(diagnostic).unwrap();
        assert_eq!(public["severity"], "warning");
        assert!(!public["remedy"].as_str().unwrap().is_empty());
        assert!(!public.to_string().contains("sensitive-"));
    }
}

#[test]
fn task_environment_cannot_case_alias_global_environment() {
    let registry = Registry::default();
    for (spelling, succeeds) in [("TOKEN", true), ("token", false)] {
        let parsed = source("project", Scope::Project, &format!("[env]\nTOKEN='base'\n[tasks.check]\nargv=['tool']\n[tasks.check.env]\n{spelling}='task'\n"));
        let result = resolve(
            &[parsed],
            &registry,
            false,
            &Selection::default(),
            Constraints::default(),
            false,
        );
        assert_eq!(result.is_ok(), succeeds);
    }
}

#[test]
fn edit_revalidation_rejects_oversized_concurrent_source_without_replacing_it() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("oyzu.toml");
    std::fs::write(&path, "[build]\njobs=2\n").unwrap();
    let edit = oyzu::config::edit::Edit::read(&path).unwrap();
    let concurrent = vec![b' '; 1024 * 1024 + 1];
    std::fs::write(&path, &concurrent).unwrap();
    let error = edit
        .commit(&path, Scope::Project, false, &Registry::default())
        .unwrap_err();
    assert!(error.to_string().contains("CONFIG_EDIT_CONFLICT"));
    assert_eq!(std::fs::read(&path).unwrap(), concurrent);
    assert!(oyzu::config::edit::Edit::read(&path)
        .err()
        .unwrap()
        .to_string()
        .contains("CONFIG_LIMIT"));
}

#[test]
fn project_capture_enforces_depth_and_aggregate_bytes_before_resolution() {
    let root = tempfile::tempdir().unwrap();
    let registry = Registry::default();
    let mut target = root.path().to_path_buf();
    for _ in 0..33 {
        target.push("n");
        std::fs::create_dir(&target).unwrap();
    }
    let error =
        oyzu::config::sources::project_sources(root.path(), &target, false, &registry).unwrap_err();
    assert!(error.to_string().contains("CONFIG_LIMIT"));
    assert!(oyzu::config::sources::project_sources(
        root.path(),
        target.parent().unwrap(),
        false,
        &registry
    )
    .is_ok());
    let comment = format!("#{}", " ".repeat(1024 * 1024 - 1));
    let mut target = root.path().to_path_buf();
    for index in 0..9 {
        if index > 0 {
            target.push("n");
        }
        std::fs::write(target.join("oyzu.toml"), &comment).unwrap();
        if index == 7 {
            assert_eq!(
                oyzu::config::sources::project_sources(root.path(), &target, false, &registry)
                    .unwrap()
                    .len(),
                8
            );
        }
    }
    let error =
        oyzu::config::sources::project_sources(root.path(), &target, false, &registry).unwrap_err();
    assert!(error.to_string().contains("CONFIG_LIMIT"));
}

#[test]
fn aggregate_entry_budget_includes_inactive_and_unknown_syntax() {
    let text = format!(
        "[profiles.inactive.future]\nvalues=[{}]\n",
        vec!["0"; 5000].join(",")
    );
    let first = source("first", Scope::Project, &text);
    let second = source("second", Scope::Local, &text);
    let check = |sources: &[ConfigSource]| {
        resolve(
            sources,
            &Registry::default(),
            false,
            &Selection::default(),
            Constraints::default(),
            false,
        )
    };
    assert!(check(std::slice::from_ref(&first)).is_ok());
    assert!(check(&[first, second])
        .unwrap_err()
        .to_string()
        .contains("CONFIG_LIMIT"));
}

#[test]
fn project_capture_rejects_aggregate_entries_before_resolution() {
    let root = tempfile::tempdir().unwrap();
    let text = format!("[future]\nvalues=[{}]\n", vec!["0"; 5000].join(","));
    std::fs::write(root.path().join("oyzu.toml"), &text).unwrap();
    std::fs::write(root.path().join("oyzu.local.toml"), &text).unwrap();
    let capture = |local| {
        oyzu::config::sources::project_sources(
            root.path(),
            root.path(),
            local,
            &Registry::default(),
        )
    };
    assert!(capture(false).is_ok());
    assert!(capture(true)
        .unwrap_err()
        .to_string()
        .contains("CONFIG_LIMIT"));
}
