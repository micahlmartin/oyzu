use super::*;

#[test]
fn assembled_index_is_complete_deterministic_and_preserves_native_blobs() {
    let temp = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();
    let mut digests = Vec::new();
    let mut before = Vec::new();
    for arch in ["amd64", "arm64"] {
        let path = temp.path().join(format!("{arch}.tar"));
        let mut files = BTreeMap::new();
        let descriptor = image(&mut files, arch, true, false);
        layout(&mut files, &descriptor);
        archive(&files, &path, false);
        before.push(fs::read(&path).unwrap());
        paths.push(path);
        digests.push(descriptor["digest"].as_str().unwrap().to_owned());
    }
    let platforms: Vec<crate::platform::Platform> = ["linux/amd64", "linux/arm64"]
        .iter()
        .map(|p| p.parse().unwrap())
        .collect();
    let mut inputs: Vec<_> = (0..2)
        .map(|i| Input {
            path: &paths[i],
            platform: &platforms[i],
            digest: &digests[i],
        })
        .collect();
    let first = temp.path().join("first.tar");
    let index = assemble(&inputs, &first).unwrap();
    assert_eq!(index.kind, "oci-index");
    assert_eq!(
        index.images.values().cloned().collect::<BTreeSet<_>>(),
        digests.iter().cloned().collect()
    );
    inputs.reverse();
    let second = temp.path().join("second.tar");
    assert_eq!(assemble(&inputs, &second).unwrap().digest, index.digest);
    assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
    for i in 0..2 {
        assert_eq!(fs::read(&paths[i]).unwrap(), before[i]);
    }
    // Shared compressed layers occur only once, although both manifests refer to them.
    let mut layout = super::super::archive::Layout::read(&first).unwrap();
    let root: Index = layout.json("index.json").unwrap();
    let document: Value = layout
        .json(&layout.descriptor(&root.manifests[0]).unwrap())
        .unwrap();
    assert_eq!(document["manifests"].as_array().unwrap().len(), 2);
    assert!(document["manifests"]
        .as_array()
        .unwrap()
        .iter()
        .all(|d| d.get("data").is_none() && d["platform"].get("variant").is_none()));
    let count = tar::Archive::new(fs::File::open(&first).unwrap())
        .entries()
        .unwrap()
        .count();
    assert_eq!(count, 8); // two manifests, two configs, one layer, index blob and layout metadata
    assert!(assemble(&inputs, &first).is_err());
    assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
}

#[test]
fn assembly_rejects_wrong_platform_digest_duplicates_and_incomplete_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("image.tar");
    let mut files = BTreeMap::new();
    let descriptor = image(&mut files, "amd64", false, false);
    layout(&mut files, &descriptor);
    archive(&files, &path, false);
    let digest = descriptor["digest"].as_str().unwrap();
    let amd64 = "linux/amd64".parse().unwrap();
    let arm64 = "linux/arm64".parse().unwrap();
    let output = temp.path().join("index.tar");
    assert!(assemble(&[], &output).is_err());
    assert!(assemble(
        &[Input {
            path: &path,
            platform: &arm64,
            digest
        }],
        &output
    )
    .is_err());
    assert!(assemble(
        &[Input {
            path: &path,
            platform: &amd64,
            digest: "sha256:wrong"
        }],
        &output
    )
    .is_err());
    assert!(assemble(
        &[
            Input {
                path: &path,
                platform: &amd64,
                digest
            },
            Input {
                path: &path,
                platform: &amd64,
                digest
            }
        ],
        &output
    )
    .is_err());
    files.remove(&format!("blobs/sha256/{}", &digest[7..]));
    archive(&files, &path, false);
    assert!(assemble(
        &[Input {
            path: &path,
            platform: &amd64,
            digest
        }],
        &output
    )
    .is_err());
    assert!(!output.exists());
}
