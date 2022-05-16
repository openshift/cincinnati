#![feature(test)]

extern crate test;
use test::Bencher;

extern crate clear_on_drop;
use clear_on_drop::{clear_stack_on_return, clear_stack_on_return_fnonce};

#[bench]
fn clear_stack_on_return_tiny(b: &mut Bencher) {
    b.iter(|| clear_stack_on_return(1, || 0x41))
}

#[bench]
fn clear_stack_on_return_small(b: &mut Bencher) {
    b.iter(|| clear_stack_on_return(2, || 0x41))
}

#[bench]
fn clear_stack_on_return_fnonce_tiny(b: &mut Bencher) {
    b.iter(|| clear_stack_on_return_fnonce(1, || 0x41))
}

#[bench]
fn clear_stack_on_return_fnonce_small(b: &mut Bencher) {
    b.iter(|| clear_stack_on_return_fnonce(2, || 0x41))
}
