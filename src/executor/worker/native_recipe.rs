//! Native executor conformance, opt-in after explicitly provisioning BuildKit.
use super::*;
use crate::executor::{
    recipe::{Base, Copy, Recipe},
    Profile,
};
use std::io::Read;

#[test]
#[ignore = "requires explicitly provisioned Linux Docker and the isolated BuildKit image"]
fn generated_recipe_builds_reproducible_native_oci_without_a_source_dockerfile() {
    let image = crate::executor::resolve_for(
        "oyzu-toolchain/docker:buildkit0.25.0",
        Profile::RootlessBuildkit,
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("source");
    fs::create_dir_all(workspace.join("input")).unwrap();
    fs::write(workspace.join("input/app"), "captured application fixture").unwrap();
    let before = snapshot::inspect_tree(&workspace).unwrap().digest;
    let recipe = Recipe {
        base: Base::Scratch,
        copies: vec![Copy {
            source: "input/app".into(),
            destination: "/app/app".into(),
        }],
        uid: 65532,
        gid: 65532,
        workdir: "/app".into(),
        entrypoint: vec!["/app/app".into()],
    };
    let mode = Mode::Buildkit {
        output: "image.oci.tar".into(),
        image_name: "oyzu/recipe:fixture".into(),
        context_files: vec!["input/app".into()],
        // This test constructs a frozen executor fixture, not user configuration.
        // Production profile selection belongs to the configuration engine.
        apparmor_profile: std::env::var("OYZU_TEST_RECIPE_APPARMOR_PROFILE")
            .expect("set OYZU_TEST_RECIPE_APPARMOR_PROFILE to the provisioned test profile"),
        dockerfile_digest: recipe.digest(&[]).unwrap(),
        generated_recipe: Some(Box::new(recipe)),
        dependency_context: None,
        images: vec![],
    };
    let platform = "linux/amd64".parse().unwrap();
    let evidence = std::env::var_os("OYZU_RECIPE_EVIDENCE_DIR").map(std::path::PathBuf::from);
    if let Some(directory) = &evidence {
        fs::create_dir_all(directory).unwrap();
        fs::write(
            directory.join("execution-mode.json"),
            serde_json::to_vec_pretty(&mode).unwrap(),
        )
        .unwrap();
    }
    let mut digests = Vec::new();
    for attempt in 0..2 {
        let output = root.path().join(format!("output-{attempt}"));
        fs::create_dir(&output).unwrap();
        let stdout = root.path().join(format!("stdout-{attempt}"));
        let stderr = root.path().join(format!("stderr-{attempt}"));
        let result = crate::executor::execute_mode(
            Request {
                log: crate::logging::Log::default(),
                image: &image,
                workspace: &workspace,
                output: &output,
                cwd: "/workspace/.",
                argv: &mode.argv("linux/amd64").unwrap(),
                env: &Default::default(),
                stdout: &stdout,
                stderr: &stderr,
                timeout: Duration::from_secs(180),
                name: &format!("oyzu-recipe-{}-{attempt}", std::process::id()),
            },
            &[],
            &mode,
            &[],
            &platform,
        )
        .unwrap();
        if let Some(directory) = &evidence {
            fs::copy(&stdout, directory.join(format!("{attempt}.stdout"))).unwrap();
            fs::copy(&stderr, directory.join(format!("{attempt}.stderr"))).unwrap();
        }
        assert_eq!(result.code, 0, "{}", fs::read_to_string(&stderr).unwrap());
        let path = output.join("image.oci.tar");
        if let Some(directory) = &evidence {
            fs::copy(&path, directory.join(format!("{attempt}.oci.tar"))).unwrap();
        }
        let verified = crate::oci::verify(&path).unwrap();
        verified.require_target(&platform).unwrap();
        digests.push((verified.digest, snapshot::file_digest(&path).unwrap()));
        let mut found = false;
        let mut payload_found = false;
        for entry in tar::Archive::new(fs::File::open(&path).unwrap())
            .entries()
            .unwrap()
        {
            let mut entry = entry.unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            if let Ok(config) = serde_json::from_slice::<Value>(&bytes) {
                if config.get("rootfs").is_some() {
                    assert_eq!(config["config"]["User"], "65532:65532");
                    assert_eq!(config["config"]["WorkingDir"], "/app");
                    assert_eq!(
                        config["config"]["Entrypoint"],
                        serde_json::json!(["/app/app"])
                    );
                    assert!(
                        config["config"]["Cmd"].is_null()
                            || config["config"]["Cmd"] == serde_json::json!([])
                    );
                    found = true;
                }
            } else if bytes.starts_with(&[0x1f, 0x8b]) {
                let decompressed = flate2::read::GzDecoder::new(bytes.as_slice());
                for member in tar::Archive::new(decompressed).entries().unwrap() {
                    let mut member = member.unwrap();
                    if member
                        .path()
                        .unwrap()
                        .to_string_lossy()
                        .trim_start_matches("./")
                        == "app/app"
                    {
                        assert_eq!(member.header().uid().unwrap(), 65532);
                        assert_eq!(member.header().gid().unwrap(), 65532);
                        let mut contents = String::new();
                        member.read_to_string(&mut contents).unwrap();
                        assert_eq!(contents, "captured application fixture");
                        payload_found = true;
                    }
                }
            }
        }
        assert!(found, "native image configuration missing");
        assert!(payload_found, "native copied payload missing");
    }
    assert_eq!(digests[0], digests[1]);
    assert_eq!(snapshot::inspect_tree(&workspace).unwrap().digest, before);
    assert!(!workspace.join("Dockerfile").exists());
}
