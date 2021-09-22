#![feature(test)]

extern crate test;
use test::Bencher;

extern crate clear_on_drop;
use clear_on_drop::ClearOnDrop;

#[bench]
fn clear_on_drop_small(b: &mut Bencher) {
    #[derive(Default)]
    struct Data {
        _data: u64,
    }

    let mut place = Data::default();
    b.iter(|| { ClearOnDrop::new(&mut place); })
}

#[bench]
fn clear_on_drop_medium(b: &mut Bencher) {
    #[derive(Default)]
    struct Data {
        _data: [u64; 32],
    }

    let mut place = Data::default();
    b.iter(|| { ClearOnDrop::new(&mut place); })
}

#[bench]
fn clear_on_drop_large(b: &mut Bencher) {
    #[derive(Default)]
    struct Data {
        _data: [[u64; 32]; 32],
    }

    let mut place = Data::default();
    b.iter(|| { ClearOnDrop::new(&mut place); })
}
