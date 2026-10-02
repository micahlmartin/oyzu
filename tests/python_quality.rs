use oyzu::discovery;
use std::fs;

#[test]
fn quality_defaults_and_native_selections_are_independent_and_explainable() {
    let root = tempfile::tempdir().unwrap();
    let metadata = root.path().join("pyproject.toml");
    fs::write(&metadata, "[project]\nname='demo'\nversion='1.0.0'\n").unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    let target = &workspace.targets["project"];
    assert_eq!(target.discovery["linter"].selected(), "ruff");
    assert_eq!(target.discovery["formatter"].selected(), "ruff");
    for name in ["lint", "format-check"] {
        assert!(target.tasks[name].build_stage);
    }
    assert!(target.tasks["format"].mutates_source);
    assert!(!target.tasks["format"].build_stage);
    assert!(target.tasks["lint"].argv.contains(&"--no-fix".into()));

    fs::write(
        &metadata,
        "[tool.black]\nline-length=99\n[tool.ruff]\nline-length=99\n",
    )
    .unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    let target = &workspace.targets["project"];
    assert_eq!(target.discovery["linter"].selected(), "ruff");
    assert_eq!(target.discovery["formatter"].selected(), "black");
    assert_eq!(target.tasks["format-check"].argv, ["black", "--check", "."]);

    fs::write(&metadata, "[tool.black]\nline-length=99\n").unwrap();
    for file in [".flake8", "setup.cfg", "tox.ini"] {
        fs::write(
            root.path().join(file),
            "[flake8] # native configuration\nmax-line-length=99\n",
        )
        .unwrap();
        let workspace = discovery::discover(root.path()).unwrap();
        let target = &workspace.targets["project"];
        assert_eq!(target.discovery["linter"].selected(), "flake8");
        assert_eq!(target.tasks["lint"].argv, ["flake8", "."]);
        assert!(serde_json::to_string(&target.discovery["linter"])
            .unwrap()
            .contains(file));
        fs::remove_file(root.path().join(file)).unwrap();
    }
}

#[test]
fn conflicting_quality_configuration_requires_resolution() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("pyproject.toml"),
        "[tool.black]\n[tool.ruff.format]\n",
    )
    .unwrap();
    let error = discovery::discover(root.path()).unwrap_err().to_string();
    assert!(error.contains("Python formatter"), "{error}");
    fs::write(
        root.path().join("pyproject.toml"),
        "[project]\nname='demo'\n",
    )
    .unwrap();
    fs::write(root.path().join("ruff.toml"), "line-length=99\n").unwrap();
    fs::write(root.path().join(".flake8"), "[flake8]\n").unwrap();
    let error = discovery::discover(root.path()).unwrap_err().to_string();
    assert!(error.contains("Python linter"), "{error}");
    fs::remove_file(root.path().join(".flake8")).unwrap();
    fs::write(root.path().join("ruff.toml"), "not valid toml[").unwrap();
    assert!(discovery::discover(root.path()).is_err());
}

#[test]
fn explicit_ruff_formatter_can_coexist_with_flake8_lint() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("pyproject.toml"), "[tool.ruff.format]\n").unwrap();
    fs::write(root.path().join(".flake8"), "[flake8]\n").unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    let target = &workspace.targets["project"];
    assert_eq!(target.discovery["linter"].selected(), "flake8");
    assert_eq!(target.discovery["formatter"].selected(), "ruff");
}
