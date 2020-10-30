extern crate dkregistry;
extern crate mockito;
extern crate tokio;

use self::mockito::mock;
use self::tokio::runtime::Runtime;

static API_VERSION_K: &'static str = "Docker-Distribution-API-Version";
static API_VERSION_V: &'static str = "registry/2.0";

#[test]
#[ignore]
fn test_base_no_insecure() {
    let addr = mockito::server_address().to_string();
    let _m = mock("GET", "/v2/")
        .with_status(200)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    // This relies on the fact that mockito is HTTP-only and
    // trying to speak TLS to it results in garbage/errors.
    runtime.block_on(futcheck).unwrap_err();

    mockito::reset();
}

#[test]
#[ignore]
fn test_base_useragent() {
    let addr = mockito::server_address().to_string();
    let _m = mock("GET", "/v2/")
        .match_header("user-agent", dkregistry::USER_AGENT)
        .with_status(200)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = runtime.block_on(futcheck).unwrap();
    assert_eq!(res, true);

    mockito::reset();
}

#[test]
fn test_base_custom_useragent() {
    let ua = "custom-ua/1.0";

    let addr = mockito::server_address().to_string();
    let _m = mock("GET", "/v2/")
        .match_header("user-agent", ua)
        .with_status(200)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .user_agent(Some(ua.to_string()))
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = runtime.block_on(futcheck).unwrap();
    assert_eq!(res, true);

    mockito::reset();
}
