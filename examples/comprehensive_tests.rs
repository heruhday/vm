use std::process;

use vm::test_js_suite::{TranslationKind, suite_cases};

fn main() {
    println!("=== Comprehensive Bytecode Tests ===\n");

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for case in suite_cases() {
        let kind = match case.translation {
            TranslationKind::Faithful => "faithful",
            TranslationKind::Lowered => "lowered",
            TranslationKind::Unsupported => "unsupported",
        };

        match case.runner {
            Some(run) => match run() {
                Ok(()) => {
                    passed += 1;
                    println!("[PASS] {} ({kind})", case.name);
                    println!("       {}", case.note);
                }
                Err(err) => {
                    failed += 1;
                    println!("[FAIL] {} ({kind})", case.name);
                    println!("       {}", case.note);
                    println!("       {}", err);
                }
            },
            None => {
                skipped += 1;
                println!("[SKIP] {} ({kind})", case.name);
                println!("       {}", case.note);
            }
        }
    }

    println!("\nSummary: {passed} passed, {failed} failed, {skipped} skipped");

    if failed > 0 {
        process::exit(1);
    }
}
