# `struct RGB` for [Rust](https://www.rust-lang.org)  [![v](https://img.shields.io/crates/v/rgb.svg)](https://crates.io/crates/rgb)

Operating on pixels as weakly-typed vectors of `u8` is error-prone and inconvenient. It's better to use vectors of pixel structs. However, Rust is so strongly typed that *your* RGB pixel struct is not compatible with *my* RGB pixel struct. So let's all use mine :P

[![xkcd standards](https://imgs.xkcd.com/comics/standards.png)](https://xkcd.com/927/)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rgb = "0.8"
```

## Usage

### `RGB` and `RGBA` structs

The structs implement common Rust traits and a few convenience functions, e.g. `map` that repeats an operation on every subpixel:

```rust
extern crate rgb;
use rgb::*; // Laziest way to use traits which add extra methods to the structs

let px = RGB {
    r:255_u8,
    g:0,
    b:255,
};
let inverted = px.map(|ch| 255 - ch);

println!("{}", inverted); // Display: rgb(0,255,0)
assert_eq!(RGB8::new(0, 255, 0), inverted);
```

### Byte slices to pixel slices

For interoperability with functions operating on generic arrays of bytes there are functinos for safe casting to and from pixel slices.

```rust
let raw = vec![0u8; width*height*3];
let pixels: &[RGB8] = raw.as_rgb(); /// Safe casts without copying
let raw_again = pixels.as_bytes();
```


----

## About colorspaces

This crate is intentionally ignorant about flavors of RGB color spaces. *Correct* color management is a complex problem, and this crate aims to be the lowest common denominator.

However, it supports any subpixel type for `RGB<T>`, and `RGBA<RGBType, AlphaType>`, so you can use them with a newtype, e.g.:

```rust
struct LinearLight(u16);
type LinearRGB = RGB<LinearLight>;
```

