use vm::test_js_suite::{TranslationKind, suite_cases};

#[test]
fn translated_cases_from_test_js_run_cleanly() {
    for case in suite_cases() {
        if let Some(run) = case.runner {
            run().unwrap_or_else(|err| panic!("{} failed: {}", case.name, err));
        }
    }
}

#[test]
fn unsupported_cases_are_tracked_explicitly() {
    let unsupported: Vec<_> = suite_cases()
        .into_iter()
        .filter(|case| case.translation == TranslationKind::Unsupported)
        .map(|case| case.name)
        .collect();

    assert_eq!(unsupported, vec!["Deterministic Fuzzer"]);
}
