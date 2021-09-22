extern crate cc;

fn main() {
    if !cfg!(feature = "no_cc") {
        cc::Build::new()
            .file("src/hide.c")
            .compile("clear_on_drop");
    }
}
