#[test]
fn annotation_contracts_reject_invalid_declarations() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
