extern crate jobserver;

use std::sync::mpsc;

use jobserver::Client;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {}", stringify!($e), e),
    })
}

#[test]
fn helper_smoke() {
    let client = t!(Client::new(1));
    drop(client.clone().into_helper_thread(|_| ()).unwrap());
    drop(client.clone().into_helper_thread(|_| ()).unwrap());
    drop(client.clone().into_helper_thread(|_| ()).unwrap());
    drop(client.clone().into_helper_thread(|_| ()).unwrap());
    drop(client.clone().into_helper_thread(|_| ()).unwrap());
    drop(client.clone().into_helper_thread(|_| ()).unwrap());
}

#[test]
fn acquire() {
    let (tx, rx) = mpsc::channel();
    let client = t!(Client::new(1));
    let helper = client.into_helper_thread(move |a| drop(tx.send(a))).unwrap();
    assert!(rx.try_recv().is_err());
    helper.request_token();
    rx.recv().unwrap().unwrap();
    helper.request_token();
    rx.recv().unwrap().unwrap();

    helper.request_token();
    helper.request_token();
    rx.recv().unwrap().unwrap();
    rx.recv().unwrap().unwrap();

    helper.request_token();
    helper.request_token();
    drop(helper);
}
