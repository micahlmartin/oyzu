//! Real named-context consumption; package-manager acquisition is separate.
use super::*;
use crate::{dependencies, executor};
use serde_json::json;
use std::io::Read;

#[test]
#[ignore = "requires provisioned Linux Docker, Alpine and the isolated BuildKit image"]
fn named_dependency_context_is_exact_readonly_offline_and_reproducible() {
    let worker = executor::resolve_for(
        "oyzu-toolchain/docker:buildkit0.25.0",
        executor::Profile::RootlessBuildkit,
    )
    .unwrap();
    let platform = "linux/amd64".parse().unwrap();
    let root = tempfile::tempdir().unwrap();
    let prepared = root.path().join("prepared");
    let store = prepared.join("contexts/packages");
    let source = root.path().join("source");
    fs::create_dir_all(&store).unwrap();
    fs::create_dir(&source).unwrap();
    let payload = b"captured package fixture bytes";
    fs::write(store.join("fixture-package.txt"), payload).unwrap();
    fs::write(source.join("source.txt"), payload).unwrap();
    fs::write(
        prepared.join("private-not-exported"),
        b"UNEXPORTED_CONTEXT_SIBLING_9cfecb",
    )
    .unwrap();
    let images = dependencies::images::capture(
        dependencies::images::Capture {
            destination: &prepared,
            image: &worker,
            target_platform: &platform,
            execution_name: &format!("oyzu-context-capture-{}", std::process::id()),
        },
        &["oyzu-fixture/alpine:amd64".into()],
    )
    .unwrap();
    let dockerfile = format!("FROM {}\nCOPY source.txt /source.txt\nRUN --mount=type=bind,from=dependencies,target=/packages cat /packages/fixture-package.txt > /mounted.txt && stat -c %Y /packages/fixture-package.txt /packages /source.txt > /times.txt && test ! -e /packages/private-not-exported && if touch /packages/write-probe 2>/dev/null; then exit 1; fi\nCOPY --from=dependencies fixture-package.txt /copied.txt\n", images[0].name);
    fs::write(source.join("Dockerfile"), dockerfile).unwrap();
    let source_before = snapshot::inspect_tree(&source).unwrap().digest;
    let prepared_before = snapshot::inspect_tree(&prepared).unwrap().digest;
    let mode: Mode = serde_json::from_value(json!({
        "kind":"buildkit", "output":"image.tar", "image_name":"oyzu/dependency-context:fixture",
        "context_files":["source.txt"], "dockerfile_digest":snapshot::file_digest(&source.join("Dockerfile")).unwrap(),
        "apparmor_profile":std::env::var("OYZU_TEST_RECIPE_APPARMOR_PROFILE").expect("provision the test AppArmor profile"),
        "images":images,
        "dependency_context":{"store":"contexts/packages", "tree_digest":snapshot::inspect_tree(&store).unwrap().digest, "platform":platform}
    })).unwrap();
    mode.validate().unwrap();
    let evidence = std::env::var_os("OYZU_RECIPE_EVIDENCE_DIR")
        .map(|p| std::path::PathBuf::from(p).join("dependency-context"));
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
        let input_time = std::time::UNIX_EPOCH + Duration::from_secs(1_800_000_000 + attempt);
        for path in [store.join("fixture-package.txt"), source.join("source.txt")] {
            fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_modified(input_time)
                .unwrap();
        }
        let output = root.path().join(format!("output-{attempt}"));
        fs::create_dir(&output).unwrap();
        let stdout = root.path().join(format!("{attempt}.stdout"));
        let stderr = root.path().join(format!("{attempt}.stderr"));
        let result = executor::execute_mode(
            Request {
                image: &worker,
                workspace: &source,
                output: &output,
                cwd: "/workspace/.",
                argv: &mode.argv("linux/amd64").unwrap(),
                env: &Default::default(),
                stdout: &stdout,
                stderr: &stderr,
                timeout: Duration::from_secs(180),
                name: &format!("oyzu-context-{}-{attempt}", std::process::id()),
            },
            &[executor::Mount {
                source: &prepared,
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
        let archive = output.join("image.tar");
        if let Some(path) = &evidence {
            fs::copy(&archive, path.join(format!("{attempt}.oci.tar"))).unwrap();
        }
        let verified = crate::oci::verify(&archive).unwrap();
        verified.require_target(&platform).unwrap();
        identities.push((verified.digest, snapshot::file_digest(&archive).unwrap()));
        let mut found = BTreeSet::new();
        for entry in tar::Archive::new(fs::File::open(&archive).unwrap())
            .entries()
            .unwrap()
        {
            let mut bytes = Vec::new();
            entry.unwrap().read_to_end(&mut bytes).unwrap();
            if !bytes.starts_with(&[0x1f, 0x8b]) {
                continue;
            }
            let layer = flate2::read::GzDecoder::new(bytes.as_slice());
            for member in tar::Archive::new(layer).entries().unwrap() {
                let mut member = member.unwrap();
                if !member.header().entry_type().is_file() {
                    continue;
                }
                let name = member
                    .path()
                    .unwrap()
                    .to_string_lossy()
                    .trim_start_matches("./")
                    .to_string();
                let mut contents = Vec::new();
                member.read_to_end(&mut contents).unwrap();
                assert!(!contents
                    .windows(b"UNEXPORTED_CONTEXT_SIBLING_9cfecb".len())
                    .any(|w| w == b"UNEXPORTED_CONTEXT_SIBLING_9cfecb"));
                if matches!(name.as_str(), "mounted.txt" | "copied.txt" | "source.txt") {
                    assert_eq!(contents, payload);
                    found.insert(name);
                } else if name == "times.txt" {
                    assert_eq!(
                        contents,
                        format!("{}\n", executor::BUILDKIT_SOURCE_DATE_EPOCH)
                            .repeat(3)
                            .as_bytes()
                    );
                    found.insert(name);
                }
            }
        }
        assert_eq!(
            found,
            BTreeSet::from([
                "copied.txt".into(),
                "mounted.txt".into(),
                "source.txt".into(),
                "times.txt".into()
            ])
        );
    }
    assert_eq!(identities[0], identities[1]);
    assert_eq!(
        snapshot::inspect_tree(&source).unwrap().digest,
        source_before
    );
    assert_eq!(
        snapshot::inspect_tree(&prepared).unwrap().digest,
        prepared_before
    );
}
