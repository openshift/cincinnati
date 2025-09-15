extern crate dkregistry;
extern crate mockito;
extern crate sha2;
extern crate tokio;

use self::mockito::mock;
use self::tokio::runtime::Runtime;
use crate::mock::blobs_download::sha2::Digest;
use futures::stream::StreamExt;
use futures::FutureExt;

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn test_blobs_has_layer() {
    let name = "my-repo/my-image";
    let digest = "fakedigest";
    let binary_digest = "binarydigest";

    let ep = format!("/v2/{}/blobs/{}", name, digest);
    let addr = mockito::server_address().to_string();
    let _m = mock("HEAD", ep.as_str())
        .with_status(200)
        .with_header("Content-Length", "0")
        .with_header("Docker-Content-Digest", binary_digest)
        .create();

    let runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.has_blob(name, digest);

    let res = runtime.block_on(futcheck).unwrap();
    assert_eq!(res, true);

    mockito::reset();
}

#[test]
fn test_blobs_hasnot_layer() {
    let name = "my-repo/my-image";
    let digest = "fakedigest";

    let ep = format!("/v2/{}/blobs/{}", name, digest);
    let addr = mockito::server_address().to_string();
    let _m = mock("HEAD", ep.as_str()).with_status(404).create();

    let runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.has_blob(name, digest);

    let res = runtime.block_on(futcheck).unwrap();
    assert_eq!(res, false);

    mockito::reset();
}

#[test]
fn get_blobs_succeeds_with_consistent_layer() -> Fallible<()> {
    let addr = mockito::server_address().to_string();

    let name = "my-repo/my-image";
    let blob = b"hello";
    let digest = format!("sha256:{:x}", sha2::Sha256::digest(blob));

    let ep = format!("/v2/{}/blobs/{}", &name, &digest);
    let _m = mock("GET", ep.as_str())
        .with_status(200)
        .with_body(blob)
        .create();

    let runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.get_blob(&name, &digest);

    let result = runtime.block_on(futcheck)?;
    assert_eq!(blob, result.as_slice());

    mockito::reset();
    Ok(())
}

#[test]
fn get_blobs_fails_with_inconsistent_layer() -> Fallible<()> {
    let addr = mockito::server_address().to_string();

    let name = "my-repo/my-image";
    let blob = b"hello";
    let blob2 = b"hello2";
    let digest = format!("sha256:{:x}", sha2::Sha256::digest(blob));

    let ep = format!("/v2/{}/blobs/{}", &name, &digest);
    let _m = mock("GET", ep.as_str())
        .with_status(200)
        .with_body(blob2)
        .create();

    let runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.get_blob(&name, &digest);

    if runtime.block_on(futcheck).is_ok() {
        return Err("expected get_blob to fail with an inconsistent blob".into());
    };

    mockito::reset();
    Ok(())
}

#[test]
fn get_blobs_stream() -> Fallible<()> {
    let addr = mockito::server_address().to_string();

    let name = "my-repo/my-image";
    let blob = b"hello";
    let digest = format!("sha256:{:x}", sha2::Sha256::digest(blob));

    let ep = format!("/v2/{}/blobs/{}", &name, &digest);
    let _m = mock("GET", ep.as_str())
        .with_status(200)
        .with_body(blob)
        .create();

    let runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.get_blob_response(&name, &digest);

    let blob_resp = runtime.block_on(futcheck)?;
    assert_eq!(blob_resp.size(), Some(5));
    let stream_output = blob_resp.stream().next().now_or_never();
    let output = stream_output.unwrap_or_else(|| panic!("No stream output"));
    let received_blob = output.unwrap_or_else(|| panic!("No blob data"))?;
    assert_eq!(blob.to_vec(), received_blob);
    mockito::reset();
    Ok(())
}
