use vm::codegen::compile_source;
use vm::js_value::to_f64;
use vm::vm::VM;

const ACC: usize = 255;

fn run_number(source: &str) -> f64 {
    let compiled = compile_source(source).expect("source should compile");
    let mut vm = VM::from_compiled(compiled, vec![]);
    vm.run(false);
    to_f64(vm.frame.regs[ACC]).expect("result should be numeric")
}

#[test]
fn json_round_trip_builtin() {
    let result = run_number(
        "let value = JSON.parse(JSON.stringify({ a: 1, b: [2, 3] })); value.a + value.b[1];",
    );
    assert_eq!(result, 4.0);
}

#[test]
fn yaml_round_trip_builtin() {
    let result = run_number(
        "let value = YAML.parse(YAML.stringify({ a: 2, b: [4, 5] })); value.a + value.b[0];",
    );
    assert_eq!(result, 6.0);
}

#[test]
fn msgpack_round_trip_builtin() {
    let result = run_number(
        "let value = Msgpack.decode(Msgpack.encode({ a: 3, b: [4, 5] })); value.a + value.b[1];",
    );
    assert_eq!(result, 8.0);
}

#[test]
fn arena_buffer_round_trip_builtin() {
    let result = run_number(
        "let value = Bin.decode(Bin.encode({ a: 7, b: [8, 9] })); value.a + value.b[0];",
    );
    assert_eq!(result, 15.0);
}

#[test]
fn date_now_builtin_returns_epoch_millis() {
    let result = run_number("let value = Date.now(); (value > 0) ? 1 : 0;");
    assert_eq!(result, 1.0);
}

#[test]
fn date_parse_and_utc_match_epoch_millis() {
    let parse_result = run_number("Date.parse('1970-01-01T00:00:01Z');");
    assert_eq!(parse_result, 1000.0);

    let utc_result = run_number("Date.UTC(1970, 0, 1, 0, 0, 1);");
    assert_eq!(utc_result, 1000.0);
}
