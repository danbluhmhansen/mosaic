#[test]
fn run_warnings() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/errors/*.rs");
}
