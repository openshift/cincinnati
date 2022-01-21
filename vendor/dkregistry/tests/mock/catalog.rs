extern crate dkregistry;
extern crate futures;
extern crate mockito;
extern crate tokio;

use self::futures::StreamExt;
use self::mockito::mock;
use self::tokio::runtime::Runtime;

#[test]
fn test_catalog_simple() {
    let repos = r#"{"repositories": ["r1/i1", "r2"]}"#;

    let ep = format!("/v2/_catalog");
    let addr = mockito::server_address().to_string();
    let _m = mock("GET", ep.as_str())
        .with_status(200)
        .with_body(repos)
        .create();

    let runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.get_catalog(None);

    let res = runtime.block_on(futcheck.map(Result::unwrap).collect::<Vec<_>>());
    assert_eq!(res, vec!["r1/i1", "r2"]);

    mockito::reset();
}

#[test]
fn test_catalog_paginate() {
    let repos_p1 = r#"{"repositories": ["r1/i1"]}"#;
    let repos_p2 = r#"{"repositories": ["r2"]}"#;

    let addr = mockito::server_address().to_string();
    let _m1 = mock("GET", "/v2/_catalog?n=1")
        .with_status(200)
        .with_header(
            "Link",
            &format!(
                r#"<{}/v2/_catalog?n=21&last=r1/i1>; rel="next""#,
                mockito::server_url()
            ),
        )
        .with_header("Content-Type", "application/json")
        .with_body(repos_p1)
        .create();
    let _m2 = mock("GET", "/v2/_catalog?n=1&last=r1")
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(repos_p2)
        .create();

    let runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let next = Box::pin(dclient.get_catalog(Some(1)));

    let (page1, next) = runtime.block_on(next.into_future());
    assert_eq!(page1.unwrap().unwrap(), "r1/i1".to_owned());

    let (page2, next) = runtime.block_on(next.into_future());
    // TODO(lucab): implement pagination
    if page2.is_some() {
        panic!("end is some: {:?}", page2);
    }

    let (end, _) = runtime.block_on(next.into_future());
    if end.is_some() {
        panic!("end is some: {:?}", end);
    }

    mockito::reset();
}
