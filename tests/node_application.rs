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
