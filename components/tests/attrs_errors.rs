#[test]
fn attrs_errors() {
    trybuild::TestCases::new().compile_fail("tests/attrs_errors/*.rs");
}
