extern crate spectral;

use dkregistry::reference::Reference;
use spectral::prelude::*;
use std::str::FromStr;

#[test]
fn valid_references() {
    struct Tcase<'a> {
        input: &'a str,
        expected_repo: &'a str,
        expected_registry: &'a str,
    }

    impl<'a> Default for Tcase<'a> {
        fn default() -> Tcase<'a> {
            Tcase {
                input: "",
                expected_repo: "library/busybox",
                expected_registry: dkregistry::reference::DEFAULT_REGISTRY,
            }
        }
    }

    for t in &[
        Tcase {
            input: "library/busybox",
            ..Default::default()
        },
        Tcase {
            input: "busybox",
            ..Default::default()
        },
        Tcase {
            input: "busybox:tag",
            ..Default::default()
        },
        Tcase {
            input: "busybox:5000",
            ..Default::default()
        },
        Tcase {
            input:
                "busybox@sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            ..Default::default()
        },
        Tcase {
            input: "quay.io/library/busybox",
            expected_registry: "quay.io",
            ..Default::default()
        },
        Tcase {
            input: "quay.io:5000/library/busybox",
            expected_registry: "quay.io:5000",
            ..Default::default()
        },
        Tcase {
            input: "1.2.3.4/library/busybox:5000",
            expected_registry: "1.2.3.4",
            ..Default::default()
        },
        Tcase {
            input: "1.2.3.4:5000/library/busybox:5000",
            expected_registry: "1.2.3.4:5000",
            ..Default::default()
        },
        Tcase {
            input: "quay.io/busybox",
            expected_registry: "quay.io",
            expected_repo: "busybox",
        },
        Tcase {
            input: "1.2.3.4/busybox:5000",
            expected_registry: "1.2.3.4",
            expected_repo: "busybox",
        },
    ] {
        let r = Reference::from_str(t.input);
        asserting(t.input).that(&r).is_ok();
        let r = r.unwrap();

        asserting(t.input)
            .that(&r.repository().as_str())
            .is_equal_to(t.expected_repo);

        asserting(t.input)
            .that(&r.registry().as_str())
            .is_equal_to(t.expected_registry);
    }
}

#[test]
fn invalid_references() {
    let tcases = vec!["".into(), "L".repeat(128), ":justatag".into()];

    for t in tcases.iter() {
        let r = Reference::from_str(t);
        asserting(t).that(&r).is_err();
    }
}

#[test]
fn hostname_without_namespace() {
    let dkr_ref = Reference::from_str(
        "sat-r220-02.lab.eng.rdu2.redhat.com:5000/default_organization-custom-ocp",
    )
    .unwrap();

    assert_eq!(
        dkr_ref.registry(),
        "sat-r220-02.lab.eng.rdu2.redhat.com:5000"
    );
    assert_eq!(dkr_ref.repository(), "default_organization-custom-ocp");
    assert_eq!(dkr_ref.version(), "latest");
}

#[test]
fn dns_registry_and_library_with_tag() -> Result<(), Box<dyn std::error::Error>> {
    let dkr_ref = Reference::from_str("quay.io/steveej/cincinnati-test-labels:0.0.0")?;

    assert_eq!(dkr_ref.registry(), "quay.io");
    assert_eq!(dkr_ref.repository(), "steveej/cincinnati-test-labels");

    Ok(())
}

#[test]
fn ipv4_registry_and_library_with_tag() -> Result<(), Box<dyn std::error::Error>> {
    let dkr_ref = Reference::from_str("1.2.3.4/steveej/cincinnati-test-labels:0.0.0")?;

    assert_eq!(dkr_ref.registry(), "1.2.3.4");
    assert_eq!(dkr_ref.repository(), "steveej/cincinnati-test-labels");

    Ok(())
}
