use oyzu::discovery;
use serde_json::json;
use std::fs;

#[test]
fn native_frontend_evidence_selects_output_without_executing_configuration() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        json!({"name":"site", "scripts":{"build":"vite build --outDir public-site"}}).to_string(),
    )
    .unwrap();
    fs::write(
        root.path().join("vite.config.ts"),
        "throw Error('static discovery must not execute this');",
    )
    .unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    let target = &workspace.targets["project"];
    assert_eq!(
        target.discovery["output-profile"].selected(),
        "vite-application"
    );
    assert_eq!(target.tasks["build"].argv, ["npm", "run", "build"]);
    assert_eq!(target.tasks["lint"].env["OYZU_NODE_BROWSER"], "1");
    assert_eq!(target.tasks["lint"].env["OYZU_NODE_VITE_CONFIG"], "1");
    assert_eq!(
        target.tasks["format-check"].env["OYZU_NODE_QUALITY_EXCLUDE"],
        "[\"public-site\"]"
    );
    fs::write(
        root.path().join("package.json"),
        json!({"name":"custom", "scripts":{"build":"node build.mjs"}}).to_string(),
    )
    .unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    assert_eq!(
        workspace.targets["project"].discovery["output-profile"].selected(),
        "native-package"
    );
}

#[test]
fn explicit_application_intent_selects_the_conventional_directory() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        json!({"name":"site", "scripts":{"build":"node build.mjs"}}).to_string(),
    )
    .unwrap();
    fs::write(
        root.path().join("build.mjs"),
        "throw Error('do not execute during discovery');",
    )
    .unwrap();
    fs::write(root.path().join("build.yaml"), "site:\n  uses: node/app\n").unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    let target = &workspace.targets["site"];
    assert_eq!(
        target.builder_selection,
        oyzu::model::BuilderSelection::Explicit
    );
    assert_eq!(
        target.discovery["output-profile"].selected(),
        "dist-application"
    );
    assert_eq!(
        target.tasks["format-check"].env["OYZU_NODE_QUALITY_EXCLUDE"],
        "[\"dist\"]"
    );
    assert!(!target.tasks["lint"].env.contains_key("OYZU_NODE_BROWSER"));
    assert_eq!(target.tasks["build"].argv, ["npm", "run", "build"]);
    fs::write(
        root.path().join("package.json"),
        json!({"name":"site", "scripts":{"build":"vite build"}}).to_string(),
    )
    .unwrap();
    assert_eq!(
        discovery::discover(root.path()).unwrap().targets["site"].discovery["output-profile"]
            .selected(),
        "vite-application"
    );
    fs::write(
        root.path().join("package.json"),
        json!({"name":"site", "workspaces":[], "scripts":{"build":"node build.mjs"}}).to_string(),
    )
    .unwrap();
    assert_eq!(
        discovery::discover(root.path()).unwrap().targets["site"].discovery["output-profile"]
            .selected(),
        "native-package"
    );
    fs::write(
        root.path().join("package.json"),
        json!({"name":"site", "scripts":{"build":"node build.mjs"}}).to_string(),
    )
    .unwrap();
    fs::write(
        root.path().join("build.yaml"),
        "site:\n  uses: node/package\n",
    )
    .unwrap();
    assert_eq!(
        discovery::discover(root.path()).unwrap().targets["site"].discovery["output-profile"]
            .selected(),
        "native-package"
    );
    fs::remove_file(root.path().join("build.yaml")).unwrap();
    assert_eq!(
        discovery::discover(root.path()).unwrap().targets["project"].builder_selection,
        oyzu::model::BuilderSelection::Inferred
    );
}
