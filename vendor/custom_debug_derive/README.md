# custom_debug

Derive `Debug` with a custom format per field.

# Usage

Here is a showcase of all possible field attributes:

```rust
    use custom_debug::Debug;
    use std::fmt;

    #[derive(Debug)]
    struct Foo {
        #[debug(format = "{} things")]
        x: i32,
        #[debug(skip)]
        y: i32,
        #[debug(with = "hex_fmt")]
        z: i32,
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
