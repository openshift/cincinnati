extern crate dkregistry;
extern crate futures;
extern crate mockito;
extern crate tokio;

use self::futures::StreamExt;
use self::mockito::mock;
use self::tokio::runtime::Runtime;

#[test]
fn test_tags_simple() {
    let name = "repo";
    let tags = r#"{"name": "repo", "tags": [ "t1", "t2" ]}"#;

    let ep = format!("/v2/{}/tags/list", name);
    let addr = mockito::server_address().to_string();
    let _m = mock("GET", ep.as_str())
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(tags)
        .create();

    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.get_tags(name, None);

    let res = runtime.block_on(futcheck.map(Result::unwrap).collect::<Vec<_>>());
    assert_eq!(res.get(0).unwrap(), &String::from("t1"));
    assert_eq!(res.get(1).unwrap(), &String::from("t2"));

    mockito::reset();
}

#[test]
fn test_tags_paginate() {
    let name = "repo";
    let tags_p1 = r#"{"name": "repo", "tags": [ "t1" ]}"#;
    let tags_p2 = r#"{"name": "repo", "tags": [ "t2" ]}"#;

    let ep1 = format!("/v2/{}/tags/list?n=1", name);
    let ep2 = format!("/v2/{}/tags/list?n=1&next_page=t1", name);
    let addr = mockito::server_address().to_string();
    let _m1 = mock("GET", ep1.as_str())
        .with_status(200)
        .with_header(
            "Link",
            &format!(
                r#"<{}/v2/_tags?n=1&next_page=t1>; rel="next""#,
                mockito::server_url()
            ),
        )
        .with_header("Content-Type", "application/json")
        .with_body(tags_p1)
        .create();
    let _m2 = mock("GET", ep2.as_str())
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(tags_p2)
        .create();

    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let next = Box::pin(dclient.get_tags(name, Some(1)).map(Result::unwrap));

    let (first_tag, stream_rest) = runtime.block_on(next.into_future());
    assert_eq!(first_tag.unwrap(), "t1".to_owned());

    let (second_tag, stream_rest) = runtime.block_on(stream_rest.into_future());
    assert_eq!(second_tag.unwrap(), "t2".to_owned());

    let (end, _) = runtime.block_on(stream_rest.into_future());
    if end.is_some() {
        panic!("end is some: {:?}", end);
    }

    mockito::reset();
}

#[test]
fn test_tags_404() {
    let name = "repo";
    let ep = format!("/v2/{}/tags/list", name);
    let addr = mockito::server_address().to_string();
    let _m = mock("GET", ep.as_str())
        .with_status(404)
        .with_header("Content-Type", "application/json")
        .create();

    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.get_tags(name, None);

    let res = runtime.block_on(futcheck.collect::<Vec<_>>());
    assert!(res.get(0).unwrap().is_err());

    mockito::reset();
}

#[test]
fn test_tags_missing_header() {
    let name = "repo";
    let tags = r#"{"name": "repo", "tags": [ "t1", "t2" ]}"#;
    let ep = format!("/v2/{}/tags/list", name);

    let addr = mockito::server_address().to_string();
    let _m = mock("GET", ep.as_str())
        .with_status(200)
        .with_body(tags)
        .create();

    let mut runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.get_tags(name, None);

    let res = runtime.block_on(futcheck.map(Result::unwrap).collect::<Vec<_>>());
    assert_eq!(vec!["t1", "t2"], res);

    mockito::reset();
}
