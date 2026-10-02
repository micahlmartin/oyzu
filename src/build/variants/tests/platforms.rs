use super::*;

const CHAIN: &str = "image: {uses: node/package, path: image, matrix: {platform: [linux/amd64, linux/arm64]}, depends_on: [checks], materialize: [{from: api, to: inputs/api.tgz}]}\napi: {uses: node/package, path: api, materialize: [{from: library, to: inputs/lib.tgz}]}\nlibrary: {uses: node/package, path: library}\nchecks: {uses: node/package, path: checks}\n";

#[test]
fn unused_standalone_ambiguity_does_not_block_explicit_platform_chains() {
    let yaml = CHAIN.replace(
        "path: library}",
        "path: library, matrix: {platform: [linux/amd64, linux/arm64]}}",
    );
    let (_root, mut workspace) = fixture(&yaml, "", &["image", "api", "library", "checks"]);
    let mut selected = Selection::new(&workspace, &["image".into()]).unwrap();
    let mut standalone = Selection::new(&workspace, &["api".into()]).unwrap();
    let mapping = expand(&mut workspace).unwrap();
    selected.expand_variants(&workspace, &mapping).unwrap();
    assert_eq!(selected.targets.len(), 7);
    assert!(planning::target_order(&workspace, &selected.targets).is_ok());
    assert!(standalone
        .expand_variants(&workspace, &mapping)
        .unwrap_err()
        .to_string()
        .contains("ambiguous runtime/platform variants"));
    assert!(
        planning::target_order(&workspace, &BTreeSet::from(["api".into()]))
            .unwrap_err()
            .to_string()
            .contains("ambiguous runtime/platform variants")
    );
}

#[test]
fn materialization_propagates_transitively_without_changing_ordering_dependencies() {
    let (_root, mut workspace) = fixture(CHAIN, "", &["image", "api", "library", "checks"]);
    let mut all = Selection::new(&workspace, &[]).unwrap();
    let mut standalone = Selection::new(&workspace, &["api".into()]).unwrap();
    let mapping = expand(&mut workspace).unwrap();
    all.expand_variants(&workspace, &mapping).unwrap();
    standalone.expand_variants(&workspace, &mapping).unwrap();
    assert_eq!(mapping["image"].len(), 2);
    assert_eq!(mapping["api"].len(), 3);
    assert_eq!(mapping["library"].len(), 3);
    assert_eq!(mapping["checks"], ["checks"]);
    assert_eq!(all.targets.len(), 7);
    assert!(!all.targets.contains("api") && !all.targets.contains("library"));
    assert_eq!(
        standalone.targets,
        BTreeSet::from(["api".into(), "library".into()])
    );
    for id in &mapping["image"] {
        let config = &workspace.declarations.targets[id];
        assert_eq!(config.depends_on, ["checks"]);
        let api = &config.materialize[0].from;
        let library = &workspace.declarations.targets[api].materialize[0].from;
        for producer in [api, library] {
            assert_eq!(
                workspace.targets[producer].variant,
                workspace.targets[id].variant
            );
            assert!(all.targets.contains(producer));
        }
    }
    assert!(planning::target_order(&workspace, &all.targets).is_ok());
}

#[test]
fn platform_tasks_keep_local_variant_identity_and_foreign_ordering_uses_default() {
    let toml =
        "[tasks.\"image:verify\"]\nargv=['echo','verify']\ndepends_on=['image:test','api:test']\n[tasks.verify]\nargv=['echo','root']\ndepends_on=['api:test']\n";
    let (_root, mut workspace) = fixture(CHAIN, toml, &["image", "api", "library", "checks"]);
    let mapping = expand(&mut workspace).unwrap();
    assert_eq!(workspace.tasks["verify"].depends_on, ["api:test"]);
    for image in &mapping["image"] {
        assert_eq!(
            workspace.tasks[&format!("{image}:verify")].depends_on,
            [format!("{image}:test"), "api:test".into()]
        );
    }
}

#[test]
fn platform_constraints_conflict_before_preparation_and_expansion_is_atomic() {
    let yaml = CHAIN.replace("path: library}", "path: library, platform: linux/amd64}");
    let (_root, mut workspace) = fixture(&yaml, "", &["image", "api", "library", "checks"]);
    let before = serde_json::to_value(&workspace).unwrap();
    let error = expand(&mut workspace).unwrap_err().to_string();
    assert!(
        error.contains("conflicts with explicit producer library"),
        "{error}"
    );
    assert_eq!(serde_json::to_value(&workspace).unwrap(), before);
}

#[test]
fn runtime_and_platform_axes_form_a_deterministic_product() {
    let yaml = "app: {uses: node/package, matrix: {node: ['22.14.0','24.14.1'], platform: [linux/arm64, linux/amd64]}}\n";
    let (_root, mut workspace) = fixture(yaml, "", &["."]);
    let mapping = expand(&mut workspace).unwrap();
    assert_eq!(mapping["app"].len(), 4);
    let combinations: BTreeSet<_> = workspace
        .targets
        .values()
        .map(|t| (t.variant["node"].clone(), t.variant["platform"].clone()))
        .collect();
    assert_eq!(combinations.len(), 4);
    for target in workspace.targets.values() {
        let builder = crate::builders::get(&target.builder).unwrap();
        assert!(builder
            .variant_toolchain(target)
            .unwrap()
            .ends_with(&format!("-node{}", target.variant["node"])));
        // Graph expansion is not authorization to execute on a foreign worker.
        let image = executor::Image {
            reference: "fixture".into(),
            digest: "fixture".into(),
            os: "linux".into(),
            arch: "amd64".into(),
        };
        assert_eq!(
            builder
                .target_platform(Some(&target.variant["platform"]), &image)
                .is_ok(),
            target.variant["platform"] == "linux/amd64"
        );
    }
    let reversed = yaml
        .replace("'22.14.0','24.14.1'", "'24.14.1','22.14.0'")
        .replace("linux/arm64, linux/amd64", "linux/amd64, linux/arm64");
    let (_root, mut other) = fixture(&reversed, "", &["."]);
    assert_eq!(expand(&mut other).unwrap(), mapping);
    let before = serde_json::to_value(&workspace).unwrap();
    expand(&mut workspace).unwrap();
    assert_eq!(serde_json::to_value(&workspace).unwrap(), before);
}

#[test]
fn authored_directory_matrix_keeps_explicit_frontend_build_standalone() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/builds/materialize-directory/project");
    let mut workspace = discovery::discover_with_shell(&root, Some("sh")).unwrap();
    let mut frontend = Selection::new(&workspace, &["frontend".into()]).unwrap();
    let mut images = Selection::new(&workspace, &["image".into()]).unwrap();
    let mapping = expand(&mut workspace).unwrap();
    frontend.expand_variants(&workspace, &mapping).unwrap();
    images.expand_variants(&workspace, &mapping).unwrap();
    assert_eq!(frontend.targets, BTreeSet::from(["frontend".into()]));
    assert_eq!(images.targets.len(), 4);
    for image in &mapping["image"] {
        let input = &workspace.declarations.targets[image].materialize[0];
        assert_eq!(
            workspace.targets[&input.from].variant,
            workspace.targets[image].variant
        );
        assert!(images.targets.contains(&input.from));
    }
}

#[test]
fn authored_named_go_outputs_inherit_platform_before_native_execution_admission() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/builds/materialize-selected-artifacts/project");
    for platform in ["linux/amd64", "linux/arm64"] {
        let mut workspace = discovery::discover_with_shell(&root, Some("sh")).unwrap();
        workspace
            .declarations
            .targets
            .get_mut("image")
            .unwrap()
            .platform = Some(platform.into());
        let mut selected = Selection::new(&workspace, &[]).unwrap();
        let mapping = expand(&mut workspace).unwrap();
        selected.expand_variants(&workspace, &mapping).unwrap();
        let inputs = &workspace.declarations.targets["image"].materialize;
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].from, inputs[1].from);
        let producer = &workspace.targets[&inputs[0].from];
        assert_eq!(producer.variant["platform"], platform);
        assert!(selected.targets.contains(&producer.name));
        assert!(!selected.targets.contains("tools"));
        let builder = crate::builders::get(&producer.builder).unwrap();
        assert!(builder.variant_toolchain(producer).is_ok());
        let worker = executor::Image {
            reference: "fixture".into(),
            digest: "fixture".into(),
            os: "linux".into(),
            arch: "amd64".into(),
        };
        let admitted = builder.target_platform(Some(platform), &worker);
        if platform == "linux/amd64" {
            assert!(admitted.is_ok());
        } else {
            assert!(admitted.unwrap_err().to_string().contains(
                "required platform linux/arm64 differs from execution platform linux/amd64"
            ));
        }
    }
}
