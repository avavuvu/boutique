use trybuild::TestCases;

#[test]
fn run_warnings() {
    let config = TestCases::new();
    config.compile_fail("tests/html_errors/*.rs");
}
