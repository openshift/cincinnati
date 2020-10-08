# Winutil

A simple library wrapping a handful of useful winapi functions.

## Usage

Add this to your Cargo.toml:

```toml
[dependencies]
winutil = "^0.1"
```

and this to your crate root:

```rust
extern crate winutil;
```

then you can use the following functions:

```rust
// Detect if the current process is running under the WoW64 subsytem.
is_wow64_process();
// Return an Option containing the NetBIOS name of the local computer.
get_computer_name();
// Return an Option containing the user associated with the current thread.
get_user_name();
```
