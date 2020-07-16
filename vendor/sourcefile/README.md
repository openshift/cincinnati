# `sourcefile`

A library for concatenating source from multiple files, whilst keeping track of where each new file
and line starts.

# Examples

Assume the following file is at `partial1.py`

```python
x = 1
y = 1
```

and that the following file is at `partial2.py`

```python
x = 1
y = 1 oops
```

then the following code

```rust
extern crate sourcefile;

use sourcefile::SourceFile;

// Assume this function exists, error is offset of unexpected char.
fn parse(source: &str) -> Result<Ast, usize> {
    // ...
}

fn main() {
    let mut source = SourceFile::new();
    source = source.add_file("./partial1.py");
    source = source.add_file("./partial2.py");

    let ast = match parse(&source.content) {
        Ok(ast) => ast,
        Err(offset) => {
            let pos = source.resolve_offset(offset);
            eprintln!("error compiling in \"{}\", line {}, col {}.", 
                      pos.filename, pos.line + 1, pos.col + 1);
        }
    }
}
```

prints

```text
error compiling in "./partial2.py", line 2, col 7.
```


