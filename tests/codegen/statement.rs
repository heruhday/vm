use gc3::*;
use vm::codegen::{CodegenError, compile_program, compile_source};

fn span() -> Span {
    Span::default()
}

fn number_expr(raw: &str) -> Expression {
    Expression::Literal(Literal::Number(NumberLiteral {
        raw: raw.to_owned(),
        span: span(),
    }))
}

fn compile_statement(statement: Statement) -> Result<(), CodegenError> {
    let program = Program {
        body: vec![statement],
        span: span(),
    };
    compile_program(&program).map(|_| ())
}

#[test]
fn codegen_compiles_throw_statement() {
    let statement = Statement::Throw(ThrowStatement {
        argument: number_expr("42"),
        span: span(),
    });

    compile_statement(statement)
        .unwrap_or_else(|error| panic!("throw should compile, got {error:?}"));
}

#[test]
fn codegen_compiles_try_catch_statement() {
    compile_source("try { throw 42; } catch (err) { err; }")
        .unwrap_or_else(|error| panic!("try/catch should compile, got {error:?}"));
}

#[test]
fn codegen_compiles_switch_statement() {
    compile_source("switch (2) { case 1: 10; break; case 2: 20; break; default: 30; }")
        .unwrap_or_else(|error| panic!("switch should compile, got {error:?}"));
}

#[test]
fn codegen_compiles_for_of_statement() {
    compile_source("for (let x of [1, 2, 3]) { x; }")
        .unwrap_or_else(|error| panic!("for-of should compile, got {error:?}"));
}

#[test]
fn codegen_rejects_try_finally_statement() {
    match compile_source("try { 1; } finally { 2; }") {
        Err(CodegenError::Unsupported { feature, .. }) => {
            assert_eq!(feature, "try/finally");
        }
        other => panic!("expected try/finally to stay unsupported, got {other:?}"),
    }
}
