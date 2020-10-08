# custom_debug_derive

Derive `Debug` with a custom format per field.

# Usage

Here is a showcase of all possible field attributes:

```rust
    #[macro_use] extern crate custom_debug_derive;
    use std::fmt;

    #[derive(CustomDebug)]
    struct Foo {
        #[debug(format = "{} things")]
        x: f32,
        #[debug(skip)]
        y: f32,
        #[debug(with = "hex_fmt")]
        z: f32,
    }

    fn hex_fmt<T: fmt::Debug>(n: &T, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:02X?}", n)
    }
```

The resulting debug output would look something like this:

```
Foo {
    x: 42 things,
    z: 0xAB
}
```
