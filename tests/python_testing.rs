use oyzu::discovery;
use std::fs;

#[test]
fn python_test_task_is_callable_without_guessing_suite_directories() {
    for (marker, contents, argv) in [
        ("requirements.txt", "", vec!["python", "-m", "pytest"]),
        ("uv.lock", "", vec!["uv", "run", "--locked", "pytest"]),
        ("poetry.lock", "", vec!["poetry", "run", "pytest"]),
    ] {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("pyproject.toml"),
            "[project]\nname='demo'\nversion='1.0.0'\n",
        )
        .unwrap();
        fs::write(root.path().join(marker), contents).unwrap();
        // Discovery must not execute code even when pytest would load it later.
        fs::write(
            root.path().join("conftest.py"),
            "raise RuntimeError('static discovery executed code')\n",
        )
        .unwrap();
        let workspace = discovery::discover(root.path()).unwrap();
        let target = &workspace.targets["project"];
        assert_eq!(target.discovery["test-framework"].selected(), "pytest");
        let task = &target.tasks["test"];
        assert_eq!(task.argv, argv);
        assert!(task.availability.is_none());
        assert!(task.build_stage);
    }
}

#[test]
fn pytest_configuration_is_evidence_without_reimplementing_native_selection() {
    for (file, text) in [
        ("pytest.ini", ""),
        (".pytest.ini", "[pytest]\ntestpaths=checks\n"),
        ("tox.ini", "[pytest]\npython_files=spec_*.py\n"),
        ("setup.cfg", "[tool:pytest]\ntestpaths=checks\n"),
        (
            "pyproject.toml",
            "[tool.pytest.ini_options]\ntestpaths=['checks']\n",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("requirements.txt"), "").unwrap();
        fs::write(root.path().join(file), text).unwrap();
        let workspace = discovery::discover(root.path()).unwrap();
        let resolution =
            serde_json::to_value(&workspace.targets["project"].discovery["test-framework"])
                .unwrap();
        assert_eq!(resolution["selected"], "pytest");
        let observation = &resolution["observations"][0]["finding"];
        assert_eq!(observation["strength"], "native");
        assert_eq!(observation["evidence"][0]["path"], file);
    }
}
