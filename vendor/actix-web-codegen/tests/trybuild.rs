#[rustversion::stable(1.54)] // MSRV
#[test]
fn compile_macros() {
    let t = trybuild::TestCases::new();

    t.pass("tests/trybuild/simple.rs");
    t.compile_fail("tests/trybuild/simple-fail.rs");

    t.pass("tests/trybuild/route-ok.rs");
    t.compile_fail("tests/trybuild/route-missing-method-fail.rs");
    t.compile_fail("tests/trybuild/route-duplicate-method-fail.rs");
    t.compile_fail("tests/trybuild/route-unexpected-method-fail.rs");
    t.compile_fail("tests/trybuild/route-malformed-path-fail.rs");

    t.pass("tests/trybuild/docstring-ok.rs");

    t.pass("tests/trybuild/test-runtime.rs");
}
