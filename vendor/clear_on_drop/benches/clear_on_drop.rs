use criterion::{criterion_group, criterion_main, Criterion};

use clear_on_drop::ClearOnDrop;

fn clear_on_drop_small(c: &mut Criterion) {
    #[derive(Default)]
    struct Data {
        _data: u64,
    }

    let mut place = Data::default();
    c.bench_function("clear_on_drop_small", |b| {
        b.iter(|| {
            ClearOnDrop::new(&mut place);
        })
    });
}

fn clear_on_drop_medium(c: &mut Criterion) {
    #[derive(Default)]
    struct Data {
        _data: [u64; 32],
    }

    let mut place = Data::default();
    c.bench_function("clear_on_drop_medium", |b| {
        b.iter(|| {
            ClearOnDrop::new(&mut place);
        })
    });
}

fn clear_on_drop_large(c: &mut Criterion) {
    #[derive(Default)]
    struct Data {
        _data: [[u64; 32]; 32],
    }

    let mut place = Data::default();
    c.bench_function("clear_on_drop_large", |b| {
        b.iter(|| {
            ClearOnDrop::new(&mut place);
        })
    });
}

criterion_group!(
    benches,
    clear_on_drop_small,
    clear_on_drop_medium,
    clear_on_drop_large
);
criterion_main!(benches);
