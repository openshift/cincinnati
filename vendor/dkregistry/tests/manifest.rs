extern crate serde_json;

use std::collections::HashMap;
use std::{fs, io};

#[test]
fn test_deserialize_manifest_v2s1_signed() {
    let f = fs::File::open("tests/fixtures/manifest_v2_s1.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let _manif: dkregistry::v2::manifest::ManifestSchema1Signed =
        serde_json::from_reader(bufrd).unwrap();
}

#[test]
fn test_deserialize_manifest_v2s2() {
    let f = fs::File::open("tests/fixtures/manifest_v2_s2.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let _manif: dkregistry::v2::manifest::ManifestSchema2Spec =
        serde_json::from_reader(bufrd).unwrap();
}

fn deserialize_manifest_v2s2_config(
) -> Result<dkregistry::v2::manifest::Manifest, Box<dyn std::error::Error>> {
    let manifest_spec = {
        let f = fs::File::open("tests/fixtures/quay.io_v2_openshift-release-dev_ocp-release_manifests_4.1.0-rc.9/application_vnd.docker.distribution.manifest.v2+json").expect("Missing fixture");

        serde_json::from_reader::<_, dkregistry::v2::manifest::ManifestSchema2Spec>(f)?
    };

    let config_blob = {
        let f = fs::File::open(format!(
            "tests/fixtures/quay.io_v2_openshift-release-dev_ocp-release_manifests_4.1.0-rc.9/{}",
            &manifest_spec.config().digest.replace(":", "_")
        ))
        .expect("Missing fixture");
        serde_json::from_reader::<_, dkregistry::v2::manifest::ConfigBlob>(f)?
    };

    Ok(dkregistry::v2::manifest::Manifest::S2(
        dkregistry::v2::manifest::ManifestSchema2 {
            manifest_spec,
            config_blob,
        },
    ))
}

#[test]
fn test_deserialize_manifest_v2s2_config() -> Result<(), Box<dyn std::error::Error>> {
    deserialize_manifest_v2s2_config()?;
    Ok(())
}

#[test]
fn test_manifest_v2s2() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = deserialize_manifest_v2s2_config()?;

    assert_eq!("amd64", manifest.architectures()?[0]);
    assert_eq!(
        vec![
            "sha256:9391a94f7498d07a595f560d60350d428b1259d622e19beee61a2363edc4eb94",
            "sha256:d4fd2952f1904c1ca0c8c3201d3ac3743f023934600c634489f0f43d48e5585d",
            "sha256:2a7baf2a728185c5679ed1736467142236b271b93c9741cbd7fe7f1c611f794b",
            "sha256:38170656dfefb3fbc6c7d7b07a1ab128227144f2eaa16eb8c877fe6a0b755670",
            "sha256:05db9bb68935b217cc844ab63e11ca816adbdd8a4aeeb4066c4c8d1125817f81",
        ],
        manifest.layers_digests(Some("amd64"))?
    );

    Ok(())
}

#[test]
fn test_deserialize_manifest_list_v2() {
    let f = fs::File::open("tests/fixtures/manifest_list_v2.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let _manif: dkregistry::v2::manifest::ManifestList = serde_json::from_reader(bufrd).unwrap();
}

#[test]
fn test_deserialize_etcd_manifest() {
    let f =
        fs::File::open("tests/fixtures/quayio_coreos_etcd_latest.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let _manif: dkregistry::v2::manifest::ManifestSchema1Signed =
        serde_json::from_reader(bufrd).unwrap();
}

#[test]
fn test_labels_manifest_v2s1_signed() {
    let f = fs::File::open("tests/fixtures/manifest_v2_s1.json").expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let manif: dkregistry::v2::manifest::ManifestSchema1Signed =
        serde_json::from_reader(bufrd).unwrap();
    assert_eq!(None, manif.get_labels(0));

    let f =
        fs::File::open("tests/fixtures/quayio_steveej_cincinnati-test-labels_dkregistry-test.json")
            .expect("Missing fixture");
    let bufrd = io::BufReader::new(f);
    let manif: dkregistry::v2::manifest::ManifestSchema1Signed =
        serde_json::from_reader(bufrd).unwrap();

    let labels_0 = manif.get_labels(0).expect("Missing labels");
    let mut expected_labels_0: HashMap<String, String> = HashMap::new();
    expected_labels_0.insert("channel".into(), "beta".into());
    assert_eq!(expected_labels_0, labels_0);
    assert_eq!(None, manif.get_labels(1));
}
