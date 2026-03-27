use gc3::*;
use vm::codegen::{CodegenError, compile_program, compile_source};

fn span() -> Span {
    Span::default()
}

fn identifier(name: &str) -> Identifier {
    Identifier {
        name: name.to_owned(),
        span: span(),
    }
}

fn ident_expr(name: &str) -> Expression {
    Expression::Identifier(identifier(name))
}

fn bool_expr(value: bool) -> Expression {
    Expression::Literal(Literal::Boolean(BooleanLiteral {
        value,
        span: span(),
    }))
}

fn null_expr() -> Expression {
    Expression::Literal(Literal::Null(span()))
}

fn number_expr(raw: &str) -> Expression {
    Expression::Literal(Literal::Number(NumberLiteral {
        raw: raw.to_owned(),
        span: span(),
    }))
}

fn string_expr(value: &str) -> Expression {
    Expression::Literal(Literal::String(StringLiteral {
        value: value.to_owned(),
        span: span(),
    }))
}

fn template_expr(value: &str) -> Expression {
    Expression::Literal(Literal::Template(TemplateLiteral {
        value: value.to_owned(),
        span: span(),
    }))
}

fn regexp_expr(source: &str) -> Expression {
    let flags = RegExpFlags {
        source: String::new(),
        has_indices: false,
        global: false,
        ignore_case: false,
        multiline: false,
        dot_all: false,
        unicode: false,
        unicode_sets: false,
        sticky: false,
    };

    let pattern = RegExpPattern {
        source: source.to_owned(),
        flags: flags.clone(),
        disjunction: RegExpDisjunction {
            alternatives: vec![RegExpAlternative {
                terms: vec![RegExpTerm::Atom {
                    atom: RegExpAtom::Raw(source.to_owned()),
                    quantifier: None,
                }],
            }],
        },
        capture_count: 0,
    };

    Expression::Literal(Literal::RegExp(RegExpLiteral {
        body: source.to_owned(),
        flags: flags.source,
        pattern,
        span: span(),
    }))
}

fn function_expr() -> Expression {
    Expression::Function(Box::new(Function {
        id: None,
        params: vec![],
        body: BlockStatement {
            body: vec![],
            span: span(),
        },
        is_async: false,
        is_generator: false,
        span: span(),
    }))
}

fn arrow_expr() -> Expression {
    Expression::ArrowFunction(Box::new(ArrowFunction {
        params: vec![],
        body: ArrowBody::Expression(Box::new(number_expr("1"))),
        is_async: false,
        span: span(),
    }))
}

fn member_expr() -> Expression {
    Expression::Member(Box::new(MemberExpression {
        object: ident_expr("obj"),
        property: MemberProperty::Identifier(identifier("prop")),
        optional: false,
        span: span(),
    }))
}

fn compile_expression(expression: Expression) -> Result<(), CodegenError> {
    let program = Program {
        body: vec![Statement::Expression(ExpressionStatement {
            expression,
            span: span(),
        })],
        span: span(),
    };

    compile_program(&program).map(|_| ())
}

fn assert_compiles(name: &str, expression: Expression) {
    if let Err(error) = compile_expression(expression) {
        panic!("{name} should compile, got {error:?}");
    }
}

fn assert_unsupported(name: &str, expression: Expression, feature: &'static str) {
    match compile_expression(expression) {
        Err(CodegenError::Unsupported {
            feature: actual, ..
        }) if actual == feature => {}
        other => panic!("{name} should fail with unsupported `{feature}`, got {other:?}"),
    }
}

#[test]
fn codegen_compiles_supported_expression_variants() {
    let cases = vec![
        ("identifier", ident_expr("value")),
        ("literal_null", null_expr()),
        ("literal_boolean", bool_expr(true)),
        ("literal_number", number_expr("42")),
        ("literal_string", string_expr("hello")),
        ("template_literal", template_expr("hello")),
        ("this", Expression::This(span())),
        (
            "array",
            Expression::Array(ArrayExpression {
                elements: vec![
                    Some(ArrayElement::Expression(number_expr("1"))),
                    Some(ArrayElement::Spread {
                        argument: ident_expr("rest"),
                        span: span(),
                    }),
                    None,
                ],
                span: span(),
            }),
        ),
        (
            "object",
            Expression::Object(ObjectExpression {
                properties: vec![ObjectProperty::Property {
                    key: PropertyKey::Identifier(identifier("value")),
                    value: number_expr("1"),
                    shorthand: false,
                    kind: ObjectPropertyKind::Init,
                    span: span(),
                }],
                span: span(),
            }),
        ),
        ("function", function_expr()),
        ("arrow_function", arrow_expr()),
        (
            "unary",
            Expression::Unary(Box::new(UnaryExpression {
                operator: UnaryOperator::Negative,
                argument: number_expr("5"),
                span: span(),
            })),
        ),
        (
            "unary_delete",
            Expression::Unary(Box::new(UnaryExpression {
                operator: UnaryOperator::Delete,
                argument: ident_expr("value"),
                span: span(),
            })),
        ),
        (
            "update",
            Expression::Update(Box::new(UpdateExpression {
                operator: UpdateOperator::Increment,
                argument: ident_expr("counter"),
                prefix: true,
                span: span(),
            })),
        ),
        (
            "binary",
            Expression::Binary(Box::new(BinaryExpression {
                operator: BinaryOperator::Add,
                left: number_expr("1"),
                right: number_expr("2"),
                span: span(),
            })),
        ),
        (
            "binary_modulo",
            Expression::Binary(Box::new(BinaryExpression {
                operator: BinaryOperator::Modulo,
                left: number_expr("5"),
                right: number_expr("2"),
                span: span(),
            })),
        ),
        (
            "logical",
            Expression::Logical(Box::new(LogicalExpression {
                operator: LogicalOperator::Or,
                left: bool_expr(false),
                right: bool_expr(true),
                span: span(),
            })),
        ),
        (
            "assignment",
            Expression::Assignment(Box::new(AssignmentExpression {
                operator: AssignmentOperator::Assign,
                left: ident_expr("target"),
                right: number_expr("7"),
                span: span(),
            })),
        ),
        (
            "conditional",
            Expression::Conditional(Box::new(ConditionalExpression {
                test: bool_expr(true),
                consequent: number_expr("1"),
                alternate: number_expr("0"),
                span: span(),
            })),
        ),
        (
            "sequence",
            Expression::Sequence(SequenceExpression {
                expressions: vec![number_expr("1"), number_expr("2")],
                span: span(),
            }),
        ),
        (
            "call",
            Expression::Call(Box::new(CallExpression {
                callee: ident_expr("fn_ref"),
                arguments: vec![CallArgument::Expression(number_expr("1"))],
                optional: false,
                span: span(),
            })),
        ),
        ("member", member_expr()),
        (
            "new",
            Expression::New(Box::new(NewExpression {
                callee: ident_expr("Ctor"),
                arguments: vec![CallArgument::Expression(number_expr("1"))],
                span: span(),
            })),
        ),
    ];

    for (name, expression) in cases {
        assert_compiles(name, expression);
    }
}

#[test]
fn codegen_rejects_unsupported_expression_variants() {
    let cases = vec![
        ("super", Expression::Super(span()), "super"),
        (
            "private_identifier",
            Expression::PrivateIdentifier(identifier("secret")),
            "private identifiers",
        ),
        (
            "class",
            Expression::Class(Box::new(Class {
                decorators: vec![],
                id: None,
                super_class: None,
                body: vec![],
                span: span(),
            })),
            "class expressions",
        ),
        (
            "tagged_template",
            Expression::TaggedTemplate(Box::new(TaggedTemplateExpression {
                tag: ident_expr("tag"),
                quasi: TemplateLiteral {
                    value: "value".to_owned(),
                    span: span(),
                },
                span: span(),
            })),
            "tagged templates",
        ),
        (
            "meta_property",
            Expression::MetaProperty(Box::new(MetaPropertyExpression {
                meta: identifier("new"),
                property: identifier("target"),
                span: span(),
            })),
            "meta properties",
        ),
        (
            "yield",
            Expression::Yield(Box::new(YieldExpression {
                argument: Some(number_expr("1")),
                delegate: false,
                span: span(),
            })),
            "yield",
        ),
        (
            "await",
            Expression::Await(Box::new(AwaitExpression {
                argument: ident_expr("promise"),
                span: span(),
            })),
            "await",
        ),
    ];

    for (name, expression, feature) in cases {
        assert_unsupported(name, expression, feature);
    }
}

#[test]
fn codegen_rejects_unsupported_expression_subfeatures() {
    let cases = vec![
        ("regexp_literal", regexp_expr("a"), "regexp literals"),
        (
            "object_spread",
            Expression::Object(ObjectExpression {
                properties: vec![ObjectProperty::Spread {
                    argument: ident_expr("source"),
                    span: span(),
                }],
                span: span(),
            }),
            "object spread",
        ),
        (
            "object_getter",
            Expression::Object(ObjectExpression {
                properties: vec![ObjectProperty::Property {
                    key: PropertyKey::Identifier(identifier("value")),
                    value: function_expr(),
                    shorthand: false,
                    kind: ObjectPropertyKind::Getter,
                    span: span(),
                }],
                span: span(),
            }),
            "getters/setters in object literals",
        ),
        (
            "update_target",
            Expression::Update(Box::new(UpdateExpression {
                operator: UpdateOperator::Increment,
                argument: number_expr("1"),
                prefix: true,
                span: span(),
            })),
            "update target",
        ),
        (
            "assignment_operator",
            Expression::Assignment(Box::new(AssignmentExpression {
                operator: AssignmentOperator::ModAssign,
                left: ident_expr("value"),
                right: number_expr("2"),
                span: span(),
            })),
            "assignment operator",
        ),
        (
            "assignment_target",
            Expression::Assignment(Box::new(AssignmentExpression {
                operator: AssignmentOperator::Assign,
                left: number_expr("1"),
                right: number_expr("2"),
                span: span(),
            })),
            "assignment target",
        ),
        (
            "optional_call",
            Expression::Call(Box::new(CallExpression {
                callee: ident_expr("fn_ref"),
                arguments: vec![],
                optional: true,
                span: span(),
            })),
            "optional call",
        ),
        (
            "private_method_call",
            Expression::Call(Box::new(CallExpression {
                callee: Expression::Member(Box::new(MemberExpression {
                    object: ident_expr("obj"),
                    property: MemberProperty::PrivateName(identifier("secret")),
                    optional: false,
                    span: span(),
                })),
                arguments: vec![],
                optional: false,
                span: span(),
            })),
            "private method calls",
        ),
        (
            "optional_member",
            Expression::Member(Box::new(MemberExpression {
                object: ident_expr("obj"),
                property: MemberProperty::Identifier(identifier("prop")),
                optional: true,
                span: span(),
            })),
            "optional chaining",
        ),
        (
            "private_member",
            Expression::Member(Box::new(MemberExpression {
                object: ident_expr("obj"),
                property: MemberProperty::PrivateName(identifier("secret")),
                optional: false,
                span: span(),
            })),
            "private members",
        ),
        (
            "new_spread",
            Expression::New(Box::new(NewExpression {
                callee: ident_expr("Ctor"),
                arguments: vec![CallArgument::Spread {
                    argument: ident_expr("args"),
                    span: span(),
                }],
                span: span(),
            })),
            "spread in `new` expressions",
        ),
        (
            "computed_delete",
            Expression::Unary(Box::new(UnaryExpression {
                operator: UnaryOperator::Delete,
                argument: Expression::Member(Box::new(MemberExpression {
                    object: ident_expr("obj"),
                    property: MemberProperty::Computed {
                        expression: Box::new(number_expr("0")),
                        span: span(),
                    },
                    optional: false,
                    span: span(),
                })),
                span: span(),
            })),
            "computed delete",
        ),
    ];

    for (name, expression, feature) in cases {
        assert_unsupported(name, expression, feature);
    }
}

#[test]
fn codegen_compiles_interpolated_template_literal_source() {
    compile_source("let a = 'heru'; `hello${a}`;").unwrap_or_else(|error| {
        panic!("interpolated template literal should compile, got {error:?}")
    });
}
