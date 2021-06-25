use dkregistry::mediatypes::MediaTypes;
use futures::stream::StreamExt;
use tokio::runtime::Runtime;

static REGISTRY: &'static str = "quay.io";

fn get_env() -> Option<(String, String)> {
    let user = ::std::env::var("DKREG_QUAY_USER");
    let password = ::std::env::var("DKREG_QUAY_PASSWD");
    match (user, password) {
        (Ok(u), Ok(t)) => Some((u, t)),
        _ => None,
    }
}

#[cfg(any(feature = "test-net", feature = "test-net-private"))]
fn common_init(
    login_scope: Option<&str>,
) -> Option<(tokio::runtime::Runtime, dkregistry::v2::Client)> {
    let mut runtime = Runtime::new().unwrap();

    let (user, password, login_scope) = if let Some(login_scope) = login_scope {
        match get_env() {
            Some((user, password)) => (Some(user), Some(password), login_scope.to_string()),
            None => return None,
        }
    } else {
        (None, None, "".to_string())
    };

    let dclient = runtime
        .block_on(
            dkregistry::v2::Client::configure()
                .registry(REGISTRY)
                .insecure_registry(false)
                .username(user)
                .password(password)
                .build()
                .unwrap()
                .authenticate(&[&login_scope]),
        )
        .unwrap();

    Some((runtime, dclient))
}

#[test]
fn test_quayio_getenv() {
    if get_env().is_none() {
        println!(
            "[WARN] {}: missing DKREG_QUAY_USER / DKREG_QUAY_PASSWD",
            REGISTRY
        );
    }
}

#[test]
fn test_quayio_base() {
    let (user, password) = match get_env() {
        Some(t) => t,
        None => return,
    };

    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(Some(user))
        .password(Some(password))
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = runtime.block_on(futcheck).unwrap();
    assert_eq!(res, true);
}

#[test]
#[ignore]
fn test_quayio_insecure() {
    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = runtime.block_on(futcheck).unwrap();
    assert_eq!(res, true);
}

#[cfg(feature = "test-net-private")]
#[test]
fn test_quayio_auth_login() {
    let login_scope = "";
    let (mut runtime, dclient) = common_init(Some(&login_scope)).unwrap();

    let futlogin = dclient.is_auth();
    let res = runtime.block_on(futlogin).unwrap();
    assert_eq!(res, true);
}

#[test]
fn test_quayio_get_tags_simple() {
    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let fut_tags = dclient.get_tags(image, None);
    let tags = runtime.block_on(fut_tags.collect::<Vec<_>>());
    let has_version = tags
        .iter()
        .map(|t| t.as_ref().unwrap())
        .any(|t| t == "latest");

    assert_eq!(has_version, true);
}

#[test]
fn test_quayio_get_tags_limit() {
    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let fut_tags = dclient.get_tags(image, Some(10));
    let tags = runtime.block_on(fut_tags.collect::<Vec<_>>());
    let has_version = tags
        .iter()
        .map(|t| t.as_ref().unwrap())
        .any(|t| t == "latest");

    assert_eq!(has_version, true);
}

#[test]
fn test_quayio_get_tags_pagination() {
    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/flannel";
    let fut_tags = dclient.get_tags(image, Some(20));
    let tags = runtime.block_on(fut_tags.collect::<Vec<_>>());
    let has_version = tags
        .iter()
        .map(|t| t.as_ref().unwrap())
        .any(|t| t == "v0.10.0");

    assert_eq!(has_version, true);
}

#[cfg(feature = "test-net-private")]
#[test]
fn test_quayio_auth_tags() {
    let image = "steveej/cincinnati-test";
    let login_scope = format!("repository:{}:pull", image);
    let (mut runtime, dclient) = common_init(Some(&login_scope)).unwrap();

    let tags = runtime
        .block_on(dclient.get_tags(image, None).collect::<Vec<_>>())
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();

    let has_version = tags.iter().any(|t| t == "0.0.1");
    assert_eq!(has_version, true);
}

#[test]
fn test_quayio_has_manifest() {
    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let reference = "latest";
    let fut = dclient.has_manifest(image, reference, None);
    let has_manifest = runtime.block_on(fut).unwrap();

    assert_eq!(has_manifest, Some(MediaTypes::ManifestV2S1Signed));
}

#[cfg(feature = "test-net-private")]
#[test]
fn test_quayio_auth_manifest() {
    let image = "steveej/cincinnati-test";
    let reference = "0.0.1";
    let login_scope = format!("repository:{}:pull", image);
    let (mut runtime, dclient) = common_init(Some(&login_scope)).unwrap();

    let fut_has_manifest = dclient.has_manifest(image, reference, None);

    let has_manifest = runtime.block_on(fut_has_manifest).unwrap();
    assert_eq!(has_manifest, Some(MediaTypes::ManifestV2S1Signed));
}

#[test]
fn test_quayio_has_no_manifest() {
    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(REGISTRY)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let image = "coreos/alpine-sh";
    let reference = "clearly_bogus";
    let fut = dclient.has_manifest(image, reference, None);
    let has_manifest = runtime.block_on(fut).unwrap();

    assert_eq!(has_manifest, None);
}

#[cfg(feature = "test-net-private")]
#[test]
fn test_quayio_auth_manifestref_missing() {
    let image = "steveej/cincinnati-test";
    let tag = "no-such-tag";

    let login_scope = format!("repository:{}:pull", image);
    let (mut runtime, dclient) = common_init(Some(&login_scope)).unwrap();
    let fut_actual = async { dclient.get_manifestref(image, tag).await };
    let actual = runtime.block_on(fut_actual);
    assert!(actual.is_err());
}

#[cfg(feature = "test-net-private")]
#[test]
fn test_quayio_auth_manifestref() {
    let image = "steveej/cincinnati-test";
    let tag = "0.0.1";
    let expected =
        String::from("sha256:cc1f79c6a6fc92982a10ced91bddeefb8fbd037a01ae106a64d0a7e79d0e4813");

    let login_scope = format!("repository:{}:pull", image);
    let (mut runtime, dclient) = common_init(Some(&login_scope)).unwrap();
    let fut_actual = async { dclient.get_manifestref(image, tag).await.unwrap() };
    let actual = runtime.block_on(fut_actual).unwrap();
    assert_eq!(actual, expected);
}

#[cfg(feature = "test-net-private")]
#[test]
fn test_quayio_auth_layer_blob() {
    let image = "steveej/cincinnati-test";
    let reference = "0.0.1";
    let layer0_sha = "sha256:ef11b765159341c08891fb84fa57d4a094903dd79059a2f8af9e1c3babda74e5";
    let layer0_len: usize = 198;

    let login_scope = format!("repository:{}:pull", image);
    let (mut runtime, dclient) = common_init(Some(&login_scope)).unwrap();

    let fut_layer0_blob = async {
        let digest = dclient
            .get_manifest(image, reference)
            .await
            .and_then(|manifest| {
                let layers: Vec<String> = manifest.layers_digests(None)?;
                let num_layers = layers.len();
                assert!(num_layers == 1, "layers length: {}", num_layers);
                let digest = layers[0].clone();
                assert!(digest == layer0_sha, "layer0 digest: {}", digest);
                Ok(digest)
            })
            .unwrap();

        dclient.get_blob(&image, &digest).await
    };

    let layer0_blob = runtime.block_on(fut_layer0_blob).unwrap();
    assert_eq!(layer0_blob.len(), layer0_len);
}
