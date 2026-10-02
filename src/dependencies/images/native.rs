//! Native acquisition-to-assembly conformance; no application runtime claim.
use super::*;
use serde_json::{json, Value};
use std::io::Read;

#[test]
#[ignore = "requires provisioned Linux Docker, Alpine and the isolated BuildKit image"]
fn captured_base_feeds_generated_assembly_without_source_dockerfile() {
    let worker = executor::resolve_for(
        "oyzu-toolchain/docker:buildkit0.25.0",
        executor::Profile::RootlessBuildkit,
    )
    .unwrap();
    let platform: Platform = "linux/amd64".parse().unwrap();
    let root = tempfile::tempdir().unwrap();
    let dependencies = root.path().join("dependencies");
    let source = root.path().join("source");
    fs::create_dir(&dependencies).unwrap();
    fs::create_dir(&source).unwrap();
    fs::write(source.join("payload"), "captured application payload").unwrap();
    let before = snapshot::inspect_tree(&source).unwrap().digest;
    let references = vec!["oyzu-fixture/alpine:amd64".to_owned()];
    let inputs = capture(
        Capture {
            destination: &dependencies,
            image: &worker,
            target_platform: &platform,
            execution_name: &format!("oyzu-base-capture-{}", std::process::id()),
        },
        &references,
    )
    .unwrap();
    assert_eq!(inputs.len(), 1);
    let captured = snapshot::inspect_tree(&dependencies).unwrap().digest;
    // Expected literal bytes are independent of the renderer implementation.
    let definition = format!(
        "FROM {}\nWORKDIR /app\nCOPY --chown=65532:65532 [\"payload\",\"/app/payload\"]\nUSER 65532:65532\nENTRYPOINT [\"/bin/cat\",\"/app/payload\"]\nCMD []\n",
        inputs[0].name
    );
    use sha2::{Digest, Sha256};
    let mode: executor::Mode = serde_json::from_value(json!({
        "kind":"buildkit", "output":"image.oci.tar", "image_name":"oyzu/captured-base:fixture",
        "context_files":["payload"],
        "apparmor_profile":std::env::var("OYZU_TEST_RECIPE_APPARMOR_PROFILE")
            .expect("set OYZU_TEST_RECIPE_APPARMOR_PROFILE to the provisioned test profile"),
        "dockerfile_digest":format!("sha256:{:x}", Sha256::digest(definition)),
        "generated_recipe":{
            "base":{"kind":"captured", "reference":references[0]},
            "copies":[{"source":"payload", "destination":"/app/payload"}],
            "uid":65532, "gid":65532, "workdir":"/app",
            "entrypoint":["/bin/cat", "/app/payload"]
        },
        "images":inputs
    }))
    .unwrap();
    mode.validate().unwrap();
    let evidence = std::env::var_os("OYZU_RECIPE_EVIDENCE_DIR")
        .map(|path| std::path::PathBuf::from(path).join("captured-base"));
    if let Some(path) = &evidence {
        fs::create_dir_all(path).unwrap();
        fs::write(
            path.join("mode.json"),
            serde_json::to_vec_pretty(&mode).unwrap(),
        )
        .unwrap();
    }
    let mut identities = Vec::new();
    for attempt in 0..2 {
        let output = root.path().join(format!("output-{attempt}"));
        fs::create_dir(&output).unwrap();
        let stdout = root.path().join(format!("{attempt}.stdout"));
        let stderr = root.path().join(format!("{attempt}.stderr"));
        let result = executor::execute_mode(
            executor::Request {
                image: &worker,
                workspace: &source,
                output: &output,
                cwd: "/workspace/.",
                argv: &mode.argv("linux/amd64").unwrap(),
                env: &BTreeMap::new(),
                stdout: &stdout,
                stderr: &stderr,
                timeout: Duration::from_secs(180),
                name: &format!("oyzu-base-assembly-{}-{attempt}", std::process::id()),
            },
            &[executor::Mount {
                source: &dependencies,
                destination: "/dependencies",
                readonly: true,
            }],
            &mode,
            &[],
            &platform,
        )
        .unwrap();
        if let Some(path) = &evidence {
            fs::copy(&stdout, path.join(format!("{attempt}.stdout"))).unwrap();
            fs::copy(&stderr, path.join(format!("{attempt}.stderr"))).unwrap();
        }
        assert_eq!(result.code, 0, "{}", fs::read_to_string(&stderr).unwrap());
        let image = output.join("image.oci.tar");
        let verified = crate::oci::verify(&image).unwrap();
        verified.require_target(&platform).unwrap();
        identities.push((verified.digest, snapshot::file_digest(&image).unwrap()));
        if let Some(path) = &evidence {
            fs::copy(&image, path.join(format!("{attempt}.oci.tar"))).unwrap();
        }
        let mut payload_found = false;
        let mut runtime_found = false;
        let mut config_found = false;
        for entry in tar::Archive::new(fs::File::open(&image).unwrap())
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
                        json!(["/bin/cat", "/app/payload"])
                    );
                    assert!(
                        config["config"]["Cmd"].is_null() || config["config"]["Cmd"] == json!([])
                    );
                    config_found = true;
                }
            } else if bytes.starts_with(&[0x1f, 0x8b]) {
                for member in tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_slice()))
                    .entries()
                    .unwrap()
                {
                    let mut member = member.unwrap();
                    let path = member
                        .path()
                        .unwrap()
                        .to_string_lossy()
                        .trim_start_matches("./")
                        .to_owned();
                    if path == "bin/busybox" {
                        runtime_found = true;
                    }
                    if path == "app/payload" {
                        assert_eq!(member.header().uid().unwrap(), 65532);
                        assert_eq!(member.header().gid().unwrap(), 65532);
                        let mut payload = String::new();
                        member.read_to_string(&mut payload).unwrap();
                        assert_eq!(payload, "captured application payload");
                        payload_found = true;
                    }
                }
            }
        }
        assert!(payload_found && runtime_found && config_found);
    }
    assert_eq!(identities[0], identities[1]);
    assert_eq!(
        snapshot::inspect_tree(&dependencies).unwrap().digest,
        captured
    );
    assert_eq!(snapshot::inspect_tree(&source).unwrap().digest, before);
    assert!(!source.join("Dockerfile").exists());
}
