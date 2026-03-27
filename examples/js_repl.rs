//! Minimal JavaScript REPL for the current bytecode compiler/runtime.
//!
//! This REPL works in "source replay" mode:
//! - each successful snippet is appended to session history
//! - the full session is recompiled and rerun on every evaluation
//!
//! That keeps top-level variables and functions working across snippets
//! without having to share runtime heap objects between separate VM instances.

#[path = "js_repl/support.rs"]
mod js_repl_support;

use std::fs;
use std::io::{self, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

use gc3::parse;
use js_repl_support::{
    EvalReports, ReplOptions, build_eval_reports, format_enabled_options, parse_startup_config,
};
use vm::asm::disassemble_clean;
use vm::codegen::{CodegenError, CompiledBytecode, compile_program};
use vm::js_value::{
    JSValue, bool_from_value, is_null, is_undefined, make_undefined, object_from_value,
    string_from_value, to_f64,
};
use vm::opt::optimize_compiled;
use vm::vm::{ObjectKind, VM};

const ACC: usize = 255;
const MAX_ARRAY_PREVIEW: usize = 20;
const MAX_RENDER_DEPTH: usize = 3;

fn main() -> io::Result<()> {
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let startup = match parse_startup_config(&raw_args) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            print_cli_help();
            return Ok(());
        }
    };

    if startup.show_cli_help {
        print_cli_help();
        return Ok(());
    }

    println!("JavaScript REPL");
    println!("Type .help for commands.");
    println!("Successful snippets are appended to session history and replayed on each run.");
    println!("Current compiler/runtime support is still partial.");
    if let Some(enabled) = format_enabled_options(startup.repl_options) {
        println!("Session options: {enabled}");
    }
    if startup.exit_after_startup {
        println!("Startup mode: one-shot");
    }
    println!();

    let stdin = io::stdin();
    let mut history = Vec::<String>::new();
    let mut pending = String::new();
    let mut last_assembly = Vec::<String>::new();
    let mut committed_console_output_len = 0usize;

    if let Err(message) = handle_startup_args(
        &startup.command_args,
        &mut history,
        &mut last_assembly,
        &mut committed_console_output_len,
        startup.repl_options,
    ) {
        eprintln!("{message}");
    }

    if startup.exit_after_startup {
        return Ok(());
    }

    loop {
        if pending.is_empty() {
            print!("js> ");
        } else {
            print!("... ");
        }
        io::stdout().flush()?;

        let mut line = String::new();
        let read = stdin.read_line(&mut line)?;
        if read == 0 {
            if !pending.trim().is_empty() {
                let force_submit = true;
                match evaluate_pending(
                    &history,
                    &pending,
                    force_submit,
                    committed_console_output_len,
                    startup.repl_options,
                ) {
                    EvalStatus::NeedMoreInput => {}
                    EvalStatus::Executed {
                        rendered,
                        reports,
                        new_console_lines,
                        ..
                    } => {
                        print_eval_reports(&reports);
                        print_console_lines(&new_console_lines);
                        print_rendered_value(&rendered, &new_console_lines);
                    }
                    EvalStatus::Error(message) => eprintln!("{message}"),
                }
            }
            break;
        }

        let trimmed = line.trim();
        if pending.is_empty() && trimmed.is_empty() {
            continue;
        }

        if pending.is_empty() && trimmed.starts_with('.') {
            match trimmed {
                ".help" => print_help(),
                ".history" => print_history(&history),
                ".asm" => print_assembly(&last_assembly),
                ".undo" => undo_last(
                    &mut history,
                    &mut last_assembly,
                    &mut committed_console_output_len,
                    startup.repl_options,
                ),
                ".clear" => {
                    history.clear();
                    pending.clear();
                    last_assembly.clear();
                    committed_console_output_len = 0;
                    println!("Session cleared.");
                }
                ".exit" | ".quit" => break,
                _ if trimmed.starts_with(".load") => {
                    match load_file_command(
                        trimmed,
                        &history,
                        committed_console_output_len,
                        startup.repl_options,
                    ) {
                        Ok(Some(loaded)) => apply_loaded_snippet(
                            loaded,
                            &mut history,
                            &mut last_assembly,
                            &mut committed_console_output_len,
                        ),
                        Ok(None) => {}
                        Err(message) => eprintln!("{message}"),
                    }
                }
                _ => eprintln!("Unknown command: {trimmed}"),
            }
            continue;
        }

        if !pending.is_empty() && trimmed == ".cancel" {
            pending.clear();
            println!("Pending input discarded.");
            continue;
        }

        let force_submit = !pending.is_empty() && trimmed.is_empty();
        if !force_submit {
            pending.push_str(&line);
        }

        match evaluate_pending(
            &history,
            &pending,
            force_submit,
            committed_console_output_len,
            startup.repl_options,
        ) {
            EvalStatus::NeedMoreInput => continue,
            EvalStatus::Executed {
                rendered,
                reports,
                assembly,
                console_output_len,
                new_console_lines,
            } => {
                history.push(pending.clone());
                pending.clear();
                last_assembly = assembly;
                committed_console_output_len = console_output_len;
                print_eval_reports(&reports);
                print_console_lines(&new_console_lines);
                print_rendered_value(&rendered, &new_console_lines);
            }
            EvalStatus::Error(message) => {
                pending.clear();
                eprintln!("{message}");
            }
        }
    }

    Ok(())
}

enum EvalStatus {
    NeedMoreInput,
    Executed {
        rendered: String,
        reports: EvalReports,
        assembly: Vec<String>,
        console_output_len: usize,
        new_console_lines: Vec<String>,
    },
    Error(String),
}

struct LoadedSnippet {
    source: String,
    rendered: String,
    reports: EvalReports,
    assembly: Vec<String>,
    console_output_len: usize,
    new_console_lines: Vec<String>,
}

fn handle_startup_args(
    startup_args: &[String],
    history: &mut Vec<String>,
    last_assembly: &mut Vec<String>,
    committed_console_output_len: &mut usize,
    repl_options: ReplOptions,
) -> Result<(), String> {
    if startup_args.is_empty() {
        return Ok(());
    }

    let command = if startup_args.first().is_some_and(|arg| arg == ".load") {
        startup_args.join(" ")
    } else {
        format!(".load {}", startup_args.join(" "))
    };

    match load_file_command(
        &command,
        history,
        *committed_console_output_len,
        repl_options,
    )? {
        Some(loaded) => {
            apply_loaded_snippet(loaded, history, last_assembly, committed_console_output_len);
            Ok(())
        }
        None => Ok(()),
    }
}

fn evaluate_pending(
    history: &[String],
    pending: &str,
    force_submit: bool,
    committed_console_output_len: usize,
    repl_options: ReplOptions,
) -> EvalStatus {
    let returns_value = match pending_returns_value(pending) {
        Ok(returns_value) => returns_value,
        Err(error) if is_incomplete_parse_error(&error.message) && !force_submit => {
            return EvalStatus::NeedMoreInput;
        }
        Err(error) => return EvalStatus::Error(format!("parse error: {error}")),
    };

    let source = build_session_source(history, pending, returns_value);
    let (program, compiled) = match compile_session(&source, repl_options.optimize) {
        Ok(result) => result,
        Err(error) => {
            return EvalStatus::Error(format!("compile error: {}", format_codegen_error(&error)));
        }
    };

    let reports = build_eval_reports(&source, &program, &compiled, repl_options);
    let assembly = disassemble_clean(&compiled.bytecode, &compiled.constants);
    let mut vm = VM::from_compiled(compiled, vec![]);
    vm.set_console_echo(false);

    match catch_unwind(AssertUnwindSafe(|| {
        vm.run(false);
        vm.frame.regs[ACC]
    })) {
        Ok(result) => {
            let console_output_len = vm.console_output.len();
            let new_console_lines = vm
                .console_output
                .iter()
                .skip(committed_console_output_len)
                .cloned()
                .collect();
            EvalStatus::Executed {
                rendered: format_value(&vm, result, 0),
                reports,
                assembly,
                console_output_len,
                new_console_lines,
            }
        }
        Err(payload) => EvalStatus::Error(format!("runtime panic: {}", panic_message(payload))),
    }
}

fn load_file_command(
    command: &str,
    history: &[String],
    committed_console_output_len: usize,
    repl_options: ReplOptions,
) -> Result<Option<LoadedSnippet>, String> {
    let Some(path) = command.strip_prefix(".load") else {
        return Ok(None);
    };
    let path = strip_matching_quotes(path.trim());
    if path.is_empty() {
        return Err("usage: .load <path>".to_owned());
    }

    let source = fs::read_to_string(path).map_err(|error| format!("load error: {error}"))?;
    match evaluate_pending(
        history,
        &source,
        true,
        committed_console_output_len,
        repl_options,
    ) {
        EvalStatus::NeedMoreInput => Err("load error: file ended with incomplete input".to_owned()),
        EvalStatus::Executed {
            rendered,
            reports,
            assembly,
            console_output_len,
            new_console_lines,
        } => Ok(Some(LoadedSnippet {
            source,
            rendered,
            reports,
            assembly,
            console_output_len,
            new_console_lines,
        })),
        EvalStatus::Error(message) => Err(message),
    }
}

fn apply_loaded_snippet(
    loaded: LoadedSnippet,
    history: &mut Vec<String>,
    last_assembly: &mut Vec<String>,
    committed_console_output_len: &mut usize,
) {
    history.push(loaded.source);
    *last_assembly = loaded.assembly;
    *committed_console_output_len = loaded.console_output_len;
    print_eval_reports(&loaded.reports);
    print_console_lines(&loaded.new_console_lines);
    print_rendered_value(&loaded.rendered, &loaded.new_console_lines);
}

fn print_eval_reports(reports: &EvalReports) {
    for line in &reports.ast_lines {
        println!("{line}");
    }
    if !reports.ast_lines.is_empty() && !reports.disasm_lines.is_empty() {
        println!();
    }
    for line in &reports.disasm_lines {
        println!("{line}");
    }
    if !reports.ast_lines.is_empty() || !reports.disasm_lines.is_empty() {
        println!();
    }
}

fn print_console_lines(lines: &[String]) {
    for line in lines {
        println!("{line}");
    }
}

fn print_rendered_value(rendered: &str, new_console_lines: &[String]) {
    if should_print_rendered_value(rendered, new_console_lines) {
        println!("{rendered}");
    }
}

fn should_print_rendered_value(rendered: &str, new_console_lines: &[String]) -> bool {
    !(rendered == "undefined" && !new_console_lines.is_empty())
}

fn compile_session(
    source: &str,
    optimize: bool,
) -> Result<(gc3::Program, CompiledBytecode), CodegenError> {
    let program = parse(source).map_err(CodegenError::Parse)?;
    let mut compiled = compile_program(&program)?;
    if optimize {
        compiled = optimize_compiled(compiled);
    }
    Ok((program, compiled))
}

fn rebuild_committed_state(
    history: &[String],
    repl_options: ReplOptions,
) -> Result<(Vec<String>, usize), String> {
    if history.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let mut source = String::new();
    for snippet in history {
        source.push_str(snippet);
        append_snippet_terminator(&mut source);
    }
    source.push_str("void 0;\n");

    let (_, compiled) = compile_session(&source, repl_options.optimize)
        .map_err(|error| format!("compile error: {}", format_codegen_error(&error)))?;
    let assembly = disassemble_clean(&compiled.bytecode, &compiled.constants);
    let mut vm = VM::from_compiled(compiled, vec![]);
    vm.set_console_echo(false);
    match catch_unwind(AssertUnwindSafe(|| vm.run(false))) {
        Ok(()) => Ok((assembly, vm.console_output.len())),
        Err(payload) => Err(format!("runtime panic: {}", panic_message(payload))),
    }
}

fn strip_matching_quotes(text: &str) -> &str {
    if text.len() >= 2 {
        let bytes = text.as_bytes();
        let first = bytes[0];
        let last = bytes[text.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &text[1..text.len() - 1];
        }
    }
    text
}

fn pending_returns_value(pending: &str) -> Result<bool, gc3::ParseError> {
    let program = parse(pending)?;
    Ok(matches!(
        program.body.last(),
        Some(gc3::Statement::Expression(_))
    ))
}

fn build_session_source(history: &[String], pending: &str, returns_value: bool) -> String {
    let mut source = String::new();
    for snippet in history {
        source.push_str(snippet);
        append_snippet_terminator(&mut source);
    }
    source.push_str(pending);
    append_snippet_terminator(&mut source);
    if !returns_value {
        source.push_str("void 0;\n");
    }
    source
}

fn append_snippet_terminator(source: &mut String) {
    if !source.ends_with('\n') {
        source.push('\n');
    }
    source.push_str(";\n");
}

fn is_incomplete_parse_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("eof") || lower.contains("unterminated") || lower.contains("unexpected end")
}

fn format_codegen_error(error: &CodegenError) -> String {
    match error {
        CodegenError::Parse(parse) => parse.to_string(),
        CodegenError::Unsupported { feature, span } => format!(
            "unsupported AST feature: {feature} at line {} column {}",
            span.start.line, span.start.column
        ),
        CodegenError::NumericLiteral { raw, span } => format!(
            "invalid numeric literal `{raw}` at line {} column {}",
            span.start.line, span.start.column
        ),
        CodegenError::InvalidBreak { span } => format!(
            "`break` used outside of a loop at line {} column {}",
            span.start.line, span.start.column
        ),
        CodegenError::InvalidContinue { span } => format!(
            "`continue` used outside of a loop at line {} column {}",
            span.start.line, span.start.column
        ),
        CodegenError::RegisterOverflow { span: Some(span) } => format!(
            "temporary register overflow at line {} column {}",
            span.start.line, span.start.column
        ),
        _ => error.to_string(),
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else {
        "unknown panic payload".to_owned()
    }
}

fn format_value(vm: &VM, value: JSValue, depth: usize) -> String {
    if depth >= MAX_RENDER_DEPTH {
        return "...".to_owned();
    }

    if is_undefined(value) {
        "undefined".to_owned()
    } else if is_null(value) {
        "null".to_owned()
    } else if let Some(boolean) = bool_from_value(value) {
        boolean.to_string()
    } else if let Some(text) = string_text(vm, value) {
        format!("{text:?}")
    } else if let Some(number) = to_f64(value) {
        format_number(number)
    } else if let Some(object_ptr) = object_from_value(value) {
        unsafe {
            match &(*object_ptr).kind {
                ObjectKind::Array(array) => format_array(vm, array, depth + 1),
                ObjectKind::Function(_)
                | ObjectKind::Closure(_)
                | ObjectKind::NativeFunction(_)
                | ObjectKind::NativeClosure(_) => "[Function]".to_owned(),
                ObjectKind::Class(_) => "[Class]".to_owned(),
                ObjectKind::Iterator { .. } => "[object Iterator]".to_owned(),
                ObjectKind::Env(_) => "[object Env]".to_owned(),
                ObjectKind::Module(_) => "[object Module]".to_owned(),
                ObjectKind::Instance(_) => "[object Instance]".to_owned(),
                ObjectKind::Symbol(_) => "[object Symbol]".to_owned(),
                ObjectKind::BoolArray(_)
                | ObjectKind::Uint8Array(_)
                | ObjectKind::Int32Array(_)
                | ObjectKind::Float64Array(_)
                | ObjectKind::StringArray(_) => "[object Array]".to_owned(),
                ObjectKind::Ordinary(_) => "[object Object]".to_owned(),
            }
        }
    } else {
        format!("{value:?}")
    }
}

fn format_array(vm: &VM, array: &vm::heap::QArray, depth: usize) -> String {
    let length = array.length as usize;
    let limit = length.min(MAX_ARRAY_PREVIEW);
    let mut items = Vec::with_capacity(limit + usize::from(length > limit));

    for index in 0..limit {
        let value = array
            .elements
            .get(index)
            .copied()
            .or_else(|| {
                array
                    .sparse
                    .as_ref()
                    .and_then(|sparse| sparse.get(&index).copied())
            })
            .unwrap_or_else(make_undefined);
        items.push(format_value(vm, value, depth));
    }

    if length > limit {
        items.push("...".to_owned());
    }

    format!("[{}]", items.join(", "))
}

fn string_text<'a>(vm: &'a VM, value: JSValue) -> Option<&'a str> {
    if let Some(atom) = value.as_atom() {
        Some(vm.atoms.resolve(atom))
    } else {
        string_from_value(value).map(|string_ptr| unsafe { (*string_ptr).text(&vm.atoms) })
    }
}

fn format_number(number: f64) -> String {
    if number.is_nan() {
        "NaN".to_owned()
    } else if number.is_infinite() && number.is_sign_positive() {
        "Infinity".to_owned()
    } else if number.is_infinite() {
        "-Infinity".to_owned()
    } else if number.fract() == 0.0 {
        format!("{number:.0}")
    } else {
        number.to_string()
    }
}

fn print_cli_help() {
    println!("Usage:");
    println!("  js_repl [--ast] [--disasm] [--opt] [--once|--batch] [.load <path> | <path>]");
    println!();
    println!("Options:");
    println!("  --ast      Show the parsed AST in a readable tree");
    println!("  --disasm   Show bytecode disassembly");
    println!("  --opt      Optimize bytecode before execution");
    println!("  --once     Run startup input once and exit");
    println!("  --batch    Alias for --once");
    println!("  --help     Show this help");
    println!();
    print_help();
}

fn print_help() {
    println!("Commands:");
    println!("  .help     Show this help");
    println!("  .history  Show committed snippets");
    println!("  .load     Load and run a file");
    println!("  .asm      Show bytecode disassembly of the last successful evaluation");
    println!("  .undo     Remove the last committed snippet");
    println!("  .clear    Clear the session history");
    println!("  .cancel   Discard the current multi-line input");
    println!("  .exit     Exit the REPL");
    println!("  .quit     Exit the REPL");
    println!();
    println!("Startup options:");
    println!("  --ast     Print a full AST tree for each successful evaluation");
    println!("  --disasm  Print bytecode disassembly for each successful evaluation");
    println!("  --opt     Optimize bytecode before execution and before `.asm` output");
    println!("  --once    Execute startup `.load` / path input and exit immediately");
    println!("  --batch   Alias for `--once`");
    println!();
    println!("Multi-line input:");
    println!("  Keep typing when the parser needs more input.");
    println!("  Enter a blank line to force-submit the current buffer.");
}

fn print_history(history: &[String]) {
    if history.is_empty() {
        println!("(history is empty)");
        return;
    }

    for (index, snippet) in history.iter().enumerate() {
        println!("#{}:", index + 1);
        print!("{snippet}");
        if !snippet.ends_with('\n') {
            println!();
        }
    }
}

fn print_assembly(assembly: &[String]) {
    if assembly.is_empty() {
        println!("(no successful evaluation yet)");
        return;
    }

    for (index, line) in assembly.iter().enumerate() {
        println!("{index:4}: {line}");
    }
}

fn undo_last(
    history: &mut Vec<String>,
    last_assembly: &mut Vec<String>,
    committed_console_output_len: &mut usize,
    repl_options: ReplOptions,
) {
    if history.pop().is_some() {
        match rebuild_committed_state(history, repl_options) {
            Ok((assembly, console_output_len)) => {
                *last_assembly = assembly;
                *committed_console_output_len = console_output_len;
            }
            Err(message) => {
                last_assembly.clear();
                *committed_console_output_len = 0;
                eprintln!("warning: {message}");
            }
        }
        println!("Removed the last snippet from history.");
    } else {
        println!("History is already empty.");
    }
}

#[cfg(test)]
mod tests {
    use super::should_print_rendered_value;

    #[test]
    fn suppresses_undefined_after_console_output() {
        let new_console_lines = vec!["true".to_owned()];
        assert!(!should_print_rendered_value(
            "undefined",
            &new_console_lines
        ));
    }

    #[test]
    fn keeps_undefined_without_console_output() {
        assert!(should_print_rendered_value("undefined", &[]));
    }

    #[test]
    fn keeps_meaningful_result_after_console_output() {
        let new_console_lines = vec!["true".to_owned()];
        assert!(should_print_rendered_value("42", &new_console_lines));
    }
}
