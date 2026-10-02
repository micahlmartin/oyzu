use oyzu::discovery;
use std::fs;

#[test]
fn native_quality_configuration_is_observed_without_executing_it() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("package.json"), r#"{"name":"demo","scripts":{"lint":"custom lint","format:check":"custom format","format":"custom write"}}"#).unwrap();
    fs::write(
        root.path().join("eslint.config.mjs"),
        "throw Error('must not execute during discovery');",
    )
    .unwrap();
    fs::write(root.path().join(".prettierrc.json"), "{\"semi\":false}").unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    let target = &workspace.targets["project"];
    assert_eq!(target.discovery["linter"].selected(), "eslint");
    assert_eq!(target.discovery["formatter"].selected(), "prettier");
    assert_eq!(target.tasks["lint"].argv, ["npm", "run", "lint"]);
    assert_eq!(
        target.tasks["format:check"].argv,
        ["npm", "run", "format:check"]
    );
    assert!(!target.tasks.contains_key("format-check"));
    assert_eq!(target.tasks["format"].argv, ["npm", "run", "format"]);
    assert!(target.tasks["format"].mutates_source);
    fs::write(root.path().join("biome.json"), "{}").unwrap();
    assert!(discovery::discover(root.path()).is_err());
}
