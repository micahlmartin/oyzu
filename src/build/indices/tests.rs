use super::*;
use crate::{discovery, oci::fixtures, records};

/// Pure graph fixture and independently authored OCI bytes; these tests do not
/// claim that a native application or container worker executed.
fn fixture() -> (
    tempfile::TempDir,
    Workspace,
    super::super::variants::Mapping,
    Value,
) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Dockerfile"), "FROM scratch\n").unwrap();
    fs::write(
        root.path().join("build.yaml"),
        "image: {uses: docker/image, matrix: {platform: [linux/amd64, linux/arm64]}}\n",
    )
    .unwrap();
    let mut workspace = discovery::discover_with_shell(root.path(), Some("sh")).unwrap();
    let mapping = super::super::variants::expand(&mut workspace).unwrap();
    let mut targets = Vec::new();
    let mut artifacts = Vec::new();
    let mut actions = Vec::new();
    for target in workspace.targets.values() {
        let platform: Platform = target.variant["platform"].parse().unwrap();
        targets.push(json!({"id":target.name,"builder":"docker/image","builderDigest":format!("sha256:{}", "1".repeat(64)),"path":".","variant":target.variant,"platform":platform}));
        artifacts.push(json!({"id":format!("{}/primary",target.name),"target":target.name,"variant":target.variant,"name":"primary","producer":format!("{}:package",target.name),"kind":"oci-image","version":"0.0.0-dev.g012345678901","mediaType":"application/vnd.oci.image.manifest.v1+json","path":format!("{}/image.tar",target.name)}));
        actions.push(
            json!({"id":format!("{}:package",target.name),"target":target.name,"dependsOn":[]}),
        );
    }
    let plan = json!({"targets":targets,"artifacts":artifacts,"actions":actions,"tools":[]});
    (root, workspace, mapping, plan)
}

fn image_records(plan: &Value, bundle: &Path) -> Vec<Value> {
    plan["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["kind"] == "oci-image")
        .map(|intent| {
            let platform: Platform = plan["targets"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["id"] == intent["target"])
                .map(|t| serde_json::from_value(t["platform"].clone()).unwrap())
                .unwrap();
            let path = bundle.join(intent["path"].as_str().unwrap());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut files = BTreeMap::new();
            let root = fixtures::image(&mut files, platform.arch(), true, false);
            fixtures::layout(&mut files, &root);
            fixtures::archive(&files, &path, false);
            let mut record = intent.clone();
            record["digest"] = json!(snapshot::file_digest(&path).unwrap());
            record["size"] = json!(fs::metadata(&path).unwrap().len());
            record["ociDigest"] = root["digest"].clone();
            record
        })
        .collect()
}

#[test]
fn planning_requires_complete_compatible_families_and_binds_the_group() {
    let (_root, workspace, mapping, initial) = fixture();
    let mut complete = initial.clone();
    plan(&workspace, &mapping, &mut complete).unwrap();
    assert_eq!(complete["targets"].as_array().unwrap().len(), 3);
    let aggregate = complete["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| is_target(t))
        .unwrap();
    assert!(aggregate["platform"].is_null());
    assert_eq!(aggregate["variant"], json!({}));
    let action = complete["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| is_action(a))
        .unwrap();
    assert_eq!(action["dependsOn"].as_array().unwrap().len(), 2);
    assert_eq!(action["inputs"].as_array().unwrap().len(), 2);
    assert_eq!(aggregate["extensions"][KEY], action["extensions"][KEY]);
    let mut partial = initial.clone();
    partial["targets"].as_array_mut().unwrap().pop();
    plan(&workspace, &mapping, &mut partial).unwrap();
    assert!(!partial["actions"].as_array().unwrap().iter().any(is_action));
    let mut different_versions = initial;
    different_versions["artifacts"][0]["version"] = json!("different");
    assert!(plan(&workspace, &mapping, &mut different_versions)
        .unwrap_err()
        .to_string()
        .contains("different versions"));
}

#[test]
fn complete_index_is_collected_inspectable_and_cannot_hide_a_missing_platform() {
    let (root, workspace, mapping, mut document) = fixture();
    plan(&workspace, &mapping, &mut document).unwrap();
    let mut artifacts = image_records(&document, root.path());
    let operation = document["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| is_action(a))
        .unwrap();
    let mut outcomes: Vec<_> = document["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| json!({"id":a["id"],"target":a["target"],"status":"succeeded"}))
        .collect();
    let index = execute(operation, &document, root.path(), &artifacts, &outcomes).unwrap();
    assert_eq!(index["kind"], "oci-index");
    let file = root.path().join(index["path"].as_str().unwrap());
    let verified = crate::oci::verify(&file).unwrap();
    assert_eq!(verified.images.len(), 2);
    artifacts.push(index.clone());
    fs::write(root.path().join("envelope.json"), "{}").unwrap();
    records::write(&root.path().join("plan.json"), &document).unwrap();
    let manifest = json!({"kind":"build-manifest","status":"succeeded","planPath":"plan.json","planDigest":records::digest("oyzu.plan.v1alpha1", &document).unwrap(),"targets":document["targets"],"actions":outcomes,"artifacts":artifacts,"reports":[],"envelopePath":"envelope.json","envelopeDigest":snapshot::file_digest(&root.path().join("envelope.json")).unwrap()});
    records::write(&root.path().join("manifest.json"), &manifest).unwrap();
    crate::build::inspect(root.path()).unwrap();
    let mut absent = manifest.clone();
    absent["artifacts"].as_array_mut().unwrap().pop();
    records::write(&root.path().join("manifest.json"), &absent).unwrap();
    assert!(crate::build::inspect(root.path())
        .unwrap_err()
        .to_string()
        .contains("missing a required OCI index"));
    let partial_path = root.path().join("partial.tar");
    let input_path = root.path().join(artifacts[0]["path"].as_str().unwrap());
    let input_platform: Platform = "linux/amd64".parse().unwrap();
    let partial = crate::oci::assemble(
        &[crate::oci::Input {
            path: &input_path,
            platform: &input_platform,
            digest: artifacts[0]["ociDigest"].as_str().unwrap(),
        }],
        &partial_path,
    )
    .unwrap();
    assert!(inspect(&manifest, &index, &partial, root.path())
        .unwrap_err()
        .to_string()
        .contains("exactly the required platform"));
    let mut missing = manifest.clone();
    missing["artifacts"].as_array_mut().unwrap().remove(0);
    assert!(inspect(&missing, &index, &verified, root.path())
        .unwrap_err()
        .to_string()
        .contains("missing a unique required image"));
    outcomes[0]["status"] = json!("failed");
    assert!(
        execute(operation, &document, root.path(), &artifacts, &outcomes)
            .unwrap_err()
            .to_string()
            .contains("prerequisite did not succeed")
    );
    let schedule =
        super::super::scheduling::Schedule::new(document["actions"].as_array().unwrap(), 1)
            .unwrap();
    assert!(!schedule.permitted(2, &outcomes));
    let source = root.path().join(artifacts[0]["path"].as_str().unwrap());
    fs::write(source, "changed").unwrap();
    assert!(inspect(&manifest, &index, &verified, root.path())
        .unwrap_err()
        .to_string()
        .contains("input content changed"));
}
