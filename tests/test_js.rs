use std::collections::{HashMap, HashSet};

use vm::codegen::compile_source;
use vm::test_js_suite::{TranslationKind, suite_cases};
use vm::vm::VM;

const TEST_JS_CASE_NAMES: [&str; 12] = [
    "Opcode Coverage",
    "Control Flow Stress",
    "Nested Loops",
    "Type Coercion",
    "Register Pressure",
    "Object Stress",
    "Shape Thrashing",
    "Closure Stress",
    "Recursion (fib)",
    "Deterministic Fuzzer",
    "Mega Test",
    "Comprehensive Binary/Unary",
];

#[test]
fn test_js_sections_are_tracked_by_suite_cases() {
    let test_js = include_str!("../test.js");
    let suite_names: HashSet<_> = suite_cases().into_iter().map(|case| case.name).collect();

    for name in TEST_JS_CASE_NAMES {
        assert!(
            test_js.contains(name),
            "expected `{name}` to still be present in test.js"
        );
        assert!(
            suite_names.contains(name),
            "missing suite case for `{name}` from test.js"
        );
    }
}

#[test]
fn supported_test_js_sections_have_runners_and_only_fuzzer_is_unsupported() {
    let suite_by_name: HashMap<_, _> = suite_cases()
        .into_iter()
        .map(|case| (case.name, case))
        .collect();

    for name in TEST_JS_CASE_NAMES {
        let case = suite_by_name
            .get(name)
            .unwrap_or_else(|| panic!("missing suite case for `{name}`"));

        if name == "Deterministic Fuzzer" {
            assert_eq!(case.translation, TranslationKind::Unsupported);
            assert!(
                case.runner.is_none(),
                "`{name}` should not have a runner yet"
            );
        } else {
            assert_ne!(
                case.translation,
                TranslationKind::Unsupported,
                "`{name}` should be executable or lowered"
            );
            assert!(case.runner.is_some(), "`{name}` should have a runner");
        }
    }
}

#[test]
fn lowered_test_js_sections_keep_explanatory_notes() {
    let lowered_cases = [
        "Closure Stress",
        "Recursion (fib)",
        "Mega Test",
        "Comprehensive Binary/Unary",
    ];

    let suite_by_name: HashMap<_, _> = suite_cases()
        .into_iter()
        .map(|case| (case.name, case))
        .collect();

    for name in lowered_cases {
        let case = suite_by_name
            .get(name)
            .unwrap_or_else(|| panic!("missing lowered suite case for `{name}`"));
        assert_eq!(case.translation, TranslationKind::Lowered);
        assert!(
            !case.note.trim().is_empty(),
            "`{name}` should explain why it is lowered"
        );
    }
}

#[test]
fn test_js_direct_codegen_compiles_and_runs() {
    let source = include_str!("../test.js");
    let compiled = compile_source(source).expect("direct codegen of test.js should now succeed");
    let mut vm = VM::from_compiled(compiled, vec![]);
    vm.run(false);
    assert_eq!(
        vm.console_output,
        vec![
            "Running Bytecode VM Tests...".to_owned(),
            "✅ All tests passed!".to_owned(),
        ]
    );
}

#[test]
fn lowered_test_js_suite_runs_cleanly() {
    for case in suite_cases() {
        if let Some(run) = case.runner {
            run().unwrap_or_else(|err| panic!("{} failed: {}", case.name, err));
        }
    }
}
