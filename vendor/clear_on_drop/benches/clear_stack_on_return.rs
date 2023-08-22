use criterion::{criterion_group, criterion_main, Criterion};

use clear_on_drop::{clear_stack_on_return, clear_stack_on_return_fnonce};

fn clear_stack_on_return_tiny(c: &mut Criterion) {
    c.bench_function("clear_stack_on_return_tiny", |b| {
        b.iter(|| clear_stack_on_return(1, || 0x41))
    });
}

fn clear_stack_on_return_small(c: &mut Criterion) {
    c.bench_function("clear_stack_on_return_small", |b| {
        b.iter(|| clear_stack_on_return(2, || 0x41))
    });
}

fn clear_stack_on_return_fnonce_tiny(c: &mut Criterion) {
    c.bench_function("clear_stack_on_return_fnonce_tiny", |b| {
        b.iter(|| clear_stack_on_return_fnonce(1, || 0x41))
    });
}

fn clear_stack_on_return_fnonce_small(c: &mut Criterion) {
    c.bench_function("clear_stack_on_return_fnonce_small", |b| {
        b.iter(|| clear_stack_on_return_fnonce(2, || 0x41))
    });
}

criterion_group!(
    benches,
    clear_stack_on_return_tiny,
    clear_stack_on_return_small,
    clear_stack_on_return_fnonce_tiny,
    clear_stack_on_return_fnonce_small
);
criterion_main!(benches);
