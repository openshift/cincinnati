#![feature(test)]

extern crate futures;
extern crate futures_locks;
extern crate test;
extern crate tokio_ as tokio;

use futures::Future;
use futures::executor::spawn;
use futures_locks::*;
use test::Bencher;

/// Benchmark the speed of acquiring an uncontested `Mutex`
#[bench]
fn bench_mutex_uncontested(bench: &mut Bencher) {
    let mutex = Mutex::<()>::new(());

    bench.iter(|| {
        spawn(mutex.lock().map(|_guard| ())).wait_future().unwrap();
    });
}

/// Benchmark the speed of acquiring a contested `Mutex`
#[bench]
fn bench_mutex_contested(bench: &mut Bencher) {
    let mutex = Mutex::<()>::new(());

    bench.iter(|| {
        let fut0 = mutex.lock().map(|_guard| ());
        let fut1 = mutex.lock().map(|_guard| ());
        spawn(fut0.join(fut1)).wait_future().unwrap();
        //spawn(mutex.lock().map(|_guard| ())).wait_future();
    });
}
