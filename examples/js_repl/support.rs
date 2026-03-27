use gc3::expression_to_js;
use vm::asm::disassemble_clean;
use vm::codegen::CompiledBytecode;

const MAX_SNIPPET_CHARS: usize = 100;
const MAX_SUMMARY_CHARS: usize = 80;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplOptions {
    pub show_ast: bool,
    pub show_disasm: bool,
    pub optimize: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupConfig {
    pub repl_options: ReplOptions,
    pub command_args: Vec<String>,
    pub show_cli_help: bool,
    pub exit_after_startup: bool,
}

#[derive(Debug, Default, Clone)]
pub struct EvalReports {
    pub ast_lines: Vec<String>,
    pub disasm_lines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstPrintMode {
    Full,
    #[allow(dead_code)]
    Outline,
}

pub fn parse_startup_config(raw_args: &[String]) -> Result<StartupConfig, String> {
    let mut repl_options = ReplOptions::default();
    let mut command_args = Vec::new();
    let mut show_cli_help = false;
    let mut exit_after_startup = false;

    for arg in raw_args {
        match arg.as_str() {
            "--ast" => repl_options.show_ast = true,
            "--disasm" => repl_options.show_disasm = true,
            "--opt" => repl_options.optimize = true,
            "--once" | "--batch" => exit_after_startup = true,
            "--help" | "-h" => show_cli_help = true,
            _ if arg.starts_with("--") => return Err(format!("unknown option: {arg}")),
            _ => command_args.push(arg.clone()),
        }
    }

    Ok(StartupConfig {
        repl_options,
        command_args,
        show_cli_help,
        exit_after_startup,
    })
}

pub fn format_enabled_options(options: ReplOptions) -> Option<String> {
    let mut enabled = Vec::new();
    if options.show_ast {
        enabled.push("--ast");
    }
    if options.show_disasm {
        enabled.push("--disasm");
    }
    if options.optimize {
        enabled.push("--opt");
    }
    (!enabled.is_empty()).then(|| enabled.join(", "))
}

pub fn build_eval_reports(
    source: &str,
    program: &gc3::Program,
    compiled: &CompiledBytecode,
    repl_options: ReplOptions,
) -> EvalReports {
    let mut reports = EvalReports::default();

    if repl_options.show_ast {
        reports.ast_lines.push("== AST ==".to_owned());
        reports
            .ast_lines
            .extend(AstPrinter::new(source, AstPrintMode::Full).render_program(program));
    }

    if repl_options.show_disasm {
        reports.disasm_lines.push(if repl_options.optimize {
            "== Disasm (optimized) ==".to_owned()
        } else {
            "== Disasm ==".to_owned()
        });
        reports.disasm_lines.push("Bytecode:".to_owned());
        for (index, line) in disassemble_clean(&compiled.bytecode, &compiled.constants)
            .iter()
            .enumerate()
        {
            reports.disasm_lines.push(format!("{index:4}: {line}"));
        }
    }

    reports
}

pub struct AstPrinter<'a> {
    source: &'a str,
    mode: AstPrintMode,
    lines: Vec<String>,
}

impl<'a> AstPrinter<'a> {
    pub fn new(source: &'a str, mode: AstPrintMode) -> Self {
        Self {
            source,
            mode,
            lines: Vec::new(),
        }
    }

    pub fn render_program(mut self, program: &gc3::Program) -> Vec<String> {
        self.push_node(
            0,
            "Program",
            program.span,
            Some(format!("statements={}", program.body.len())),
            false,
        );
        for statement in &program.body {
            self.write_statement(statement, 1);
        }
        self.lines
    }

    fn push_line(&mut self, depth: usize, text: impl Into<String>) {
        self.lines
            .push(format!("{}{}", "  ".repeat(depth), text.into()));
    }

    fn push_node(
        &mut self,
        depth: usize,
        kind: &str,
        span: gc3::Span,
        detail: Option<String>,
        include_snippet: bool,
    ) {
        let mut line = format!("{}{} @ {}", "  ".repeat(depth), kind, format_span(span));
        if let Some(detail) = detail.filter(|detail| !detail.is_empty()) {
            line.push(' ');
            line.push_str(&detail);
        }
        if include_snippet && let Some(snippet) = snippet_for_span(self.source, span) {
            line.push_str(" :: ");
            line.push_str(&snippet);
        }
        self.lines.push(line);
    }

    fn write_statement(&mut self, statement: &gc3::Statement, depth: usize) {
        match statement {
            gc3::Statement::Directive(node) => self.push_node(
                depth,
                "Directive",
                node.span,
                Some(format!(
                    "value={:?}",
                    truncate_text(&node.value, MAX_SUMMARY_CHARS)
                )),
                true,
            ),
            gc3::Statement::Empty(span) => {
                self.push_node(depth, "EmptyStatement", *span, None, true)
            }
            gc3::Statement::Debugger(span) => {
                self.push_node(depth, "DebuggerStatement", *span, None, true)
            }
            gc3::Statement::Block(block) => {
                self.push_node(
                    depth,
                    "BlockStatement",
                    block.span,
                    Some(format!("items={}", block.body.len())),
                    true,
                );
                for item in &block.body {
                    self.write_statement(item, depth + 1);
                }
            }
            gc3::Statement::Labeled(node) => {
                self.push_node(
                    depth,
                    "LabeledStatement",
                    node.span,
                    Some(format!("label={}", node.label.name)),
                    true,
                );
                self.write_statement(&node.body, depth + 1);
            }
            gc3::Statement::ImportDeclaration(node) => {
                self.push_node(
                    depth,
                    "ImportDeclaration",
                    node.span,
                    Some(import_clause_summary(node)),
                    true,
                );
                if self.mode == AstPrintMode::Full
                    && let Some(attributes) = &node.attributes
                {
                    self.push_line(depth + 1, "Attributes:");
                    self.write_expression(attributes, depth + 2);
                }
            }
            gc3::Statement::ExportDeclaration(node) => {
                self.write_export_declaration(node, depth);
            }
            gc3::Statement::VariableDeclaration(node) => {
                self.write_variable_declaration(node, depth);
            }
            gc3::Statement::FunctionDeclaration(node) => {
                self.write_function("FunctionDeclaration", node, depth, true);
            }
            gc3::Statement::ClassDeclaration(node) => {
                self.write_class("ClassDeclaration", node, depth, true);
            }
            gc3::Statement::If(node) => {
                self.push_node(depth, "IfStatement", node.span, None, true);
                if self.mode == AstPrintMode::Full {
                    self.push_line(depth + 1, "Test:");
                    self.write_expression(&node.test, depth + 2);
                }
                self.push_line(depth + 1, "Consequent:");
                self.write_statement(&node.consequent, depth + 2);
                if let Some(alternate) = &node.alternate {
                    self.push_line(depth + 1, "Alternate:");
                    self.write_statement(alternate, depth + 2);
                }
            }
            gc3::Statement::While(node) => {
                self.push_node(depth, "WhileStatement", node.span, None, true);
                if self.mode == AstPrintMode::Full {
                    self.push_line(depth + 1, "Test:");
                    self.write_expression(&node.test, depth + 2);
                }
                self.push_line(depth + 1, "Body:");
                self.write_statement(&node.body, depth + 2);
            }
            gc3::Statement::DoWhile(node) => {
                self.push_node(depth, "DoWhileStatement", node.span, None, true);
                self.push_line(depth + 1, "Body:");
                self.write_statement(&node.body, depth + 2);
                if self.mode == AstPrintMode::Full {
                    self.push_line(depth + 1, "Test:");
                    self.write_expression(&node.test, depth + 2);
                }
            }
            gc3::Statement::For(node) => {
                self.write_for_statement(node, depth);
            }
            gc3::Statement::Switch(node) => {
                self.push_node(
                    depth,
                    "SwitchStatement",
                    node.span,
                    Some(format!("cases={}", node.cases.len())),
                    true,
                );
                if self.mode == AstPrintMode::Full {
                    self.push_line(depth + 1, "Discriminant:");
                    self.write_expression(&node.discriminant, depth + 2);
                }
                for (index, case) in node.cases.iter().enumerate() {
                    self.write_switch_case(case, index, depth + 1);
                }
            }
            gc3::Statement::Return(node) => {
                self.push_node(depth, "ReturnStatement", node.span, None, true);
                if self.mode == AstPrintMode::Full
                    && let Some(argument) = &node.argument
                {
                    self.push_line(depth + 1, "Argument:");
                    self.write_expression(argument, depth + 2);
                }
            }
            gc3::Statement::Break(node) => {
                let detail = node
                    .label
                    .as_ref()
                    .map(|label| format!("label={}", label.name));
                self.push_node(depth, "BreakStatement", node.span, detail, true);
            }
            gc3::Statement::Continue(node) => {
                let detail = node
                    .label
                    .as_ref()
                    .map(|label| format!("label={}", label.name));
                self.push_node(depth, "ContinueStatement", node.span, detail, true);
            }
            gc3::Statement::Throw(node) => {
                self.push_node(depth, "ThrowStatement", node.span, None, true);
                if self.mode == AstPrintMode::Full {
                    self.push_line(depth + 1, "Argument:");
                    self.write_expression(&node.argument, depth + 2);
                }
            }
            gc3::Statement::Try(node) => {
                self.push_node(
                    depth,
                    "TryStatement",
                    node.span,
                    Some(format!(
                        "catch={} finally={}",
                        node.handler.is_some(),
                        node.finalizer.is_some()
                    )),
                    true,
                );
                self.push_line(depth + 1, "Block:");
                self.write_statement(&gc3::Statement::Block(node.block.clone()), depth + 2);
                if let Some(handler) = &node.handler {
                    self.write_catch_clause(handler, depth + 1);
                }
                if let Some(finalizer) = &node.finalizer {
                    self.push_line(depth + 1, "Finalizer:");
                    self.write_statement(&gc3::Statement::Block(finalizer.clone()), depth + 2);
                }
            }
            gc3::Statement::With(node) => {
                self.push_node(depth, "WithStatement", node.span, None, true);
                if self.mode == AstPrintMode::Full {
                    self.push_line(depth + 1, "Object:");
                    self.write_expression(&node.object, depth + 2);
                }
                self.push_line(depth + 1, "Body:");
                self.write_statement(&node.body, depth + 2);
            }
            gc3::Statement::Expression(node) => {
                self.push_node(
                    depth,
                    "ExpressionStatement",
                    node.span,
                    Some(format!("expr={}", summarize_expression(&node.expression))),
                    true,
                );
                if self.mode == AstPrintMode::Full {
                    self.write_expression(&node.expression, depth + 1);
                }
            }
        }
    }

    fn write_export_declaration(&mut self, declaration: &gc3::ExportDeclaration, depth: usize) {
        match declaration {
            gc3::ExportDeclaration::All(node) => {
                self.push_node(
                    depth,
                    "ExportAllDeclaration",
                    node.span,
                    Some(format!("source={:?}", node.source.value)),
                    true,
                );
                if self.mode == AstPrintMode::Full
                    && let Some(attributes) = &node.attributes
                {
                    self.push_line(depth + 1, "Attributes:");
                    self.write_expression(attributes, depth + 2);
                }
            }
            gc3::ExportDeclaration::Named(node) => {
                self.push_node(
                    depth,
                    "ExportNamedDeclaration",
                    node.span,
                    Some(format!("specifiers={}", node.specifiers.len())),
                    true,
                );
                for specifier in &node.specifiers {
                    self.push_node(
                        depth + 1,
                        "ExportSpecifier",
                        specifier.span,
                        Some(format!(
                            "{} -> {}",
                            module_export_name_summary(&specifier.local),
                            module_export_name_summary(&specifier.exported)
                        )),
                        true,
                    );
                }
                if self.mode == AstPrintMode::Full
                    && let Some(attributes) = &node.attributes
                {
                    self.push_line(depth + 1, "Attributes:");
                    self.write_expression(attributes, depth + 2);
                }
            }
            gc3::ExportDeclaration::Default(node) => {
                self.push_node(depth, "ExportDefaultDeclaration", node.span, None, true);
                if self.mode == AstPrintMode::Full {
                    match &node.declaration {
                        gc3::ExportDefaultKind::Function(function) => {
                            self.write_function("FunctionDeclaration", function, depth + 1, true);
                        }
                        gc3::ExportDefaultKind::Class(class) => {
                            self.write_class("ClassDeclaration", class, depth + 1, true);
                        }
                        gc3::ExportDefaultKind::Expression(expression) => {
                            self.push_line(depth + 1, "Expression:");
                            self.write_expression(expression, depth + 2);
                        }
                    }
                }
            }
            gc3::ExportDeclaration::Declaration(node) => match node {
                gc3::ExportedDeclaration::Variable(variable) => {
                    self.write_variable_declaration(variable, depth);
                }
                gc3::ExportedDeclaration::Function(function) => {
                    self.write_function("FunctionDeclaration", function, depth, true);
                }
                gc3::ExportedDeclaration::Class(class) => {
                    self.write_class("ClassDeclaration", class, depth, true);
                }
            },
        }
    }

    fn write_variable_declaration(&mut self, declaration: &gc3::VariableDeclaration, depth: usize) {
        self.push_node(
            depth,
            "VariableDeclaration",
            declaration.span,
            Some(format!(
                "kind={} declarations={}",
                variable_kind_name(declaration.kind),
                declaration.declarations.len()
            )),
            true,
        );
        for declarator in &declaration.declarations {
            self.write_variable_declarator(declarator, depth + 1);
        }
    }

    fn write_variable_declarator(&mut self, declarator: &gc3::VariableDeclarator, depth: usize) {
        self.push_node(depth, "VariableDeclarator", declarator.span, None, true);
        self.push_line(depth + 1, "Pattern:");
        self.write_pattern(&declarator.pattern, depth + 2);
        if self.mode == AstPrintMode::Full
            && let Some(init) = &declarator.init
        {
            self.push_line(depth + 1, "Initializer:");
            self.write_expression(init, depth + 2);
        }
    }

    fn write_function(
        &mut self,
        label: &str,
        function: &gc3::Function,
        depth: usize,
        include_snippet: bool,
    ) {
        let name = function
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("<anonymous>");
        self.push_node(
            depth,
            label,
            function.span,
            Some(format!(
                "name={} params={} async={} generator={}",
                name,
                function.params.len(),
                function.is_async,
                function.is_generator
            )),
            include_snippet,
        );
        for (index, param) in function.params.iter().enumerate() {
            self.push_line(depth + 1, format!("Param[{index}]:"));
            self.write_pattern(param, depth + 2);
        }
        self.push_line(depth + 1, "Body:");
        self.write_statement(&gc3::Statement::Block(function.body.clone()), depth + 2);
    }

    fn write_class(
        &mut self,
        label: &str,
        class: &gc3::Class,
        depth: usize,
        include_snippet: bool,
    ) {
        let name = class
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("<anonymous>");
        self.push_node(
            depth,
            label,
            class.span,
            Some(format!(
                "name={} elements={} decorators={}",
                name,
                class.body.len(),
                class.decorators.len()
            )),
            include_snippet,
        );
        if self.mode == AstPrintMode::Full
            && let Some(super_class) = &class.super_class
        {
            self.push_line(depth + 1, "Super:");
            self.write_expression(super_class, depth + 2);
        }
        if self.mode == AstPrintMode::Full {
            for (index, decorator) in class.decorators.iter().enumerate() {
                self.push_line(depth + 1, format!("Decorator[{index}]:"));
                self.write_expression(decorator, depth + 2);
            }
        }
        for element in &class.body {
            self.write_class_element(element, depth + 1);
        }
    }

    fn write_class_element(&mut self, element: &gc3::ClassElement, depth: usize) {
        match element {
            gc3::ClassElement::Empty(span) => {
                self.push_node(depth, "ClassElement::Empty", *span, None, true);
            }
            gc3::ClassElement::StaticBlock(block) => {
                self.push_node(depth, "StaticBlock", block.span, None, true);
                self.write_statement(&gc3::Statement::Block(block.clone()), depth + 1);
            }
            gc3::ClassElement::Method(method) => {
                self.push_node(
                    depth,
                    "ClassMethod",
                    method.span,
                    Some(format!(
                        "kind={:?} key={} static={}",
                        method.kind,
                        property_key_summary(&method.key),
                        method.is_static
                    )),
                    true,
                );
                if self.mode == AstPrintMode::Full {
                    self.write_function("FunctionExpression", &method.value, depth + 1, false);
                }
            }
            gc3::ClassElement::Field(field) => {
                self.push_node(
                    depth,
                    "ClassField",
                    field.span,
                    Some(format!(
                        "key={} static={} accessor={}",
                        property_key_summary(&field.key),
                        field.is_static,
                        field.is_accessor
                    )),
                    true,
                );
                if self.mode == AstPrintMode::Full
                    && let Some(value) = &field.value
                {
                    self.push_line(depth + 1, "Value:");
                    self.write_expression(value, depth + 2);
                }
            }
        }
    }

    fn write_for_statement(&mut self, statement: &gc3::ForStatement, depth: usize) {
        match statement {
            gc3::ForStatement::Classic(node) => {
                self.push_node(depth, "ForStatement::Classic", node.span, None, true);
                if let Some(init) = &node.init {
                    self.push_line(depth + 1, "Init:");
                    match init {
                        gc3::ForInit::VariableDeclaration(declaration) => {
                            self.write_variable_declaration(declaration, depth + 2);
                        }
                        gc3::ForInit::Expression(expression) => {
                            if self.mode == AstPrintMode::Full {
                                self.write_expression(expression, depth + 2);
                            } else {
                                self.push_line(depth + 2, "(expression omitted in outline)");
                            }
                        }
                    }
                }
                if self.mode == AstPrintMode::Full
                    && let Some(test) = &node.test
                {
                    self.push_line(depth + 1, "Test:");
                    self.write_expression(test, depth + 2);
                }
                if self.mode == AstPrintMode::Full
                    && let Some(update) = &node.update
                {
                    self.push_line(depth + 1, "Update:");
                    self.write_expression(update, depth + 2);
                }
                self.push_line(depth + 1, "Body:");
                self.write_statement(&node.body, depth + 2);
            }
            gc3::ForStatement::In(node) => {
                self.push_node(
                    depth,
                    "ForStatement::In",
                    node.span,
                    Some(format!("await={}", node.is_await)),
                    true,
                );
                self.push_line(depth + 1, "Left:");
                self.write_for_left(&node.left, depth + 2);
                if self.mode == AstPrintMode::Full {
                    self.push_line(depth + 1, "Right:");
                    self.write_expression(&node.right, depth + 2);
                }
                self.push_line(depth + 1, "Body:");
                self.write_statement(&node.body, depth + 2);
            }
            gc3::ForStatement::Of(node) => {
                self.push_node(
                    depth,
                    "ForStatement::Of",
                    node.span,
                    Some(format!("await={}", node.is_await)),
                    true,
                );
                self.push_line(depth + 1, "Left:");
                self.write_for_left(&node.left, depth + 2);
                if self.mode == AstPrintMode::Full {
                    self.push_line(depth + 1, "Right:");
                    self.write_expression(&node.right, depth + 2);
                }
                self.push_line(depth + 1, "Body:");
                self.write_statement(&node.body, depth + 2);
            }
        }
    }

    fn write_for_left(&mut self, left: &gc3::ForLeft, depth: usize) {
        match left {
            gc3::ForLeft::VariableDeclaration(declaration) => {
                self.write_variable_declaration(declaration, depth);
            }
            gc3::ForLeft::Pattern(pattern) => {
                self.write_pattern(pattern, depth);
            }
            gc3::ForLeft::Expression(expression) => {
                if self.mode == AstPrintMode::Full {
                    self.write_expression(expression, depth);
                } else {
                    self.push_line(depth, "(expression omitted in outline)");
                }
            }
        }
    }

    fn write_switch_case(&mut self, case: &gc3::SwitchCase, index: usize, depth: usize) {
        let label = if case.test.is_some() {
            format!("SwitchCase[{index}]")
        } else {
            format!("SwitchDefault[{index}]")
        };
        self.push_node(
            depth,
            &label,
            case.span,
            Some(format!("consequent={}", case.consequent.len())),
            true,
        );
        if self.mode == AstPrintMode::Full
            && let Some(test) = &case.test
        {
            self.push_line(depth + 1, "Test:");
            self.write_expression(test, depth + 2);
        }
        for statement in &case.consequent {
            self.write_statement(statement, depth + 1);
        }
    }

    fn write_catch_clause(&mut self, clause: &gc3::CatchClause, depth: usize) {
        self.push_node(depth, "CatchClause", clause.span, None, true);
        if let Some(param) = &clause.param {
            self.push_line(depth + 1, "Param:");
            self.write_pattern(param, depth + 2);
        }
        self.push_line(depth + 1, "Body:");
        self.write_statement(&gc3::Statement::Block(clause.body.clone()), depth + 2);
    }

    fn write_pattern(&mut self, pattern: &gc3::Pattern, depth: usize) {
        match pattern {
            gc3::Pattern::Identifier(identifier) => self.push_node(
                depth,
                "Pattern::Identifier",
                identifier.span,
                Some(format!("name={}", identifier.name)),
                true,
            ),
            gc3::Pattern::Array(array) => {
                self.push_node(
                    depth,
                    "Pattern::Array",
                    array.span,
                    Some(format!("elements={}", array.elements.len())),
                    true,
                );
                for element in &array.elements {
                    match element {
                        Some(element) => self.write_pattern(element, depth + 1),
                        None => self.push_line(depth + 1, "Pattern::Elision"),
                    }
                }
            }
            gc3::Pattern::Object(object) => {
                self.push_node(
                    depth,
                    "Pattern::Object",
                    object.span,
                    Some(format!("properties={}", object.properties.len())),
                    true,
                );
                for property in &object.properties {
                    match property {
                        gc3::ObjectPatternProperty::Property {
                            key,
                            value,
                            shorthand,
                            span,
                        } => {
                            self.push_node(
                                depth + 1,
                                "ObjectPatternProperty",
                                *span,
                                Some(format!(
                                    "key={} shorthand={}",
                                    property_key_summary(key),
                                    shorthand
                                )),
                                true,
                            );
                            self.write_pattern(value, depth + 2);
                        }
                        gc3::ObjectPatternProperty::Rest { argument, span } => {
                            self.push_node(depth + 1, "ObjectPatternRest", *span, None, true);
                            self.write_pattern(argument, depth + 2);
                        }
                    }
                }
            }
            gc3::Pattern::Rest(rest) => {
                self.push_node(depth, "Pattern::Rest", rest.span, None, true);
                self.write_pattern(&rest.argument, depth + 1);
            }
            gc3::Pattern::Assignment(assignment) => {
                self.push_node(depth, "Pattern::Assignment", assignment.span, None, true);
                self.push_line(depth + 1, "Left:");
                self.write_pattern(&assignment.left, depth + 2);
                if self.mode == AstPrintMode::Full {
                    self.push_line(depth + 1, "Right:");
                    self.write_expression(&assignment.right, depth + 2);
                }
            }
        }
    }

    fn write_expression(&mut self, expression: &gc3::Expression, depth: usize) {
        match expression {
            gc3::Expression::Identifier(identifier) => self.push_node(
                depth,
                "Identifier",
                identifier.span,
                Some(format!("name={}", identifier.name)),
                false,
            ),
            gc3::Expression::PrivateIdentifier(identifier) => self.push_node(
                depth,
                "PrivateIdentifier",
                identifier.span,
                Some(format!("name={}", identifier.name)),
                false,
            ),
            gc3::Expression::Literal(literal) => self.write_literal(literal, depth),
            gc3::Expression::This(span) => {
                self.push_node(depth, "ThisExpression", *span, None, false)
            }
            gc3::Expression::Super(span) => {
                self.push_node(depth, "SuperExpression", *span, None, false)
            }
            gc3::Expression::Array(array) => {
                self.push_node(
                    depth,
                    "ArrayExpression",
                    array.span,
                    Some(format!("elements={}", array.elements.len())),
                    false,
                );
                for element in &array.elements {
                    match element {
                        Some(gc3::ArrayElement::Expression(expression)) => {
                            self.write_expression(expression, depth + 1);
                        }
                        Some(gc3::ArrayElement::Spread { argument, span }) => {
                            self.push_node(depth + 1, "SpreadElement", *span, None, false);
                            self.write_expression(argument, depth + 2);
                        }
                        None => self.push_line(depth + 1, "ArrayHole"),
                    }
                }
            }
            gc3::Expression::Object(object) => {
                self.push_node(
                    depth,
                    "ObjectExpression",
                    object.span,
                    Some(format!("properties={}", object.properties.len())),
                    false,
                );
                for property in &object.properties {
                    self.write_object_property(property, depth + 1);
                }
            }
            gc3::Expression::Function(function) => {
                self.write_function("FunctionExpression", function, depth, false);
            }
            gc3::Expression::ArrowFunction(function) => {
                self.push_node(
                    depth,
                    "ArrowFunctionExpression",
                    function.span,
                    Some(format!(
                        "params={} async={}",
                        function.params.len(),
                        function.is_async
                    )),
                    false,
                );
                for (index, param) in function.params.iter().enumerate() {
                    self.push_line(depth + 1, format!("Param[{index}]:"));
                    self.write_pattern(param, depth + 2);
                }
                match &function.body {
                    gc3::ArrowBody::Expression(expression) => {
                        self.push_line(depth + 1, "Body:");
                        self.write_expression(expression, depth + 2);
                    }
                    gc3::ArrowBody::Block(block) => {
                        self.push_line(depth + 1, "Body:");
                        self.write_statement(&gc3::Statement::Block(block.clone()), depth + 2);
                    }
                }
            }
            gc3::Expression::Class(class) => {
                self.write_class("ClassExpression", class, depth, false);
            }
            gc3::Expression::TaggedTemplate(node) => {
                self.push_node(
                    depth,
                    "TaggedTemplateExpression",
                    node.span,
                    Some(format!(
                        "template={:?}",
                        truncate_text(&node.quasi.value, MAX_SUMMARY_CHARS)
                    )),
                    false,
                );
                self.push_line(depth + 1, "Tag:");
                self.write_expression(&node.tag, depth + 2);
            }
            gc3::Expression::MetaProperty(node) => {
                self.push_node(
                    depth,
                    "MetaProperty",
                    node.span,
                    Some(format!("{}.{}", node.meta.name, node.property.name)),
                    false,
                );
            }
            gc3::Expression::Yield(node) => {
                self.push_node(
                    depth,
                    "YieldExpression",
                    node.span,
                    Some(format!("delegate={}", node.delegate)),
                    false,
                );
                if let Some(argument) = &node.argument {
                    self.write_expression(argument, depth + 1);
                }
            }
            gc3::Expression::Await(node) => {
                self.push_node(depth, "AwaitExpression", node.span, None, false);
                self.write_expression(&node.argument, depth + 1);
            }
            gc3::Expression::Unary(node) => {
                self.push_node(
                    depth,
                    "UnaryExpression",
                    node.span,
                    Some(format!("op={:?}", node.operator)),
                    false,
                );
                self.write_expression(&node.argument, depth + 1);
            }
            gc3::Expression::Update(node) => {
                self.push_node(
                    depth,
                    "UpdateExpression",
                    node.span,
                    Some(format!("op={:?} prefix={}", node.operator, node.prefix)),
                    false,
                );
                self.write_expression(&node.argument, depth + 1);
            }
            gc3::Expression::Binary(node) => {
                self.push_node(
                    depth,
                    "BinaryExpression",
                    node.span,
                    Some(format!(
                        "op={:?} expr={}",
                        node.operator,
                        summarize_expression(expression)
                    )),
                    false,
                );
                self.write_expression(&node.left, depth + 1);
                self.write_expression(&node.right, depth + 1);
            }
            gc3::Expression::Logical(node) => {
                self.push_node(
                    depth,
                    "LogicalExpression",
                    node.span,
                    Some(format!(
                        "op={:?} expr={}",
                        node.operator,
                        summarize_expression(expression)
                    )),
                    false,
                );
                self.write_expression(&node.left, depth + 1);
                self.write_expression(&node.right, depth + 1);
            }
            gc3::Expression::Assignment(node) => {
                self.push_node(
                    depth,
                    "AssignmentExpression",
                    node.span,
                    Some(format!(
                        "op={:?} expr={}",
                        node.operator,
                        summarize_expression(expression)
                    )),
                    false,
                );
                self.write_expression(&node.left, depth + 1);
                self.write_expression(&node.right, depth + 1);
            }
            gc3::Expression::Conditional(node) => {
                self.push_node(depth, "ConditionalExpression", node.span, None, false);
                self.push_line(depth + 1, "Test:");
                self.write_expression(&node.test, depth + 2);
                self.push_line(depth + 1, "Consequent:");
                self.write_expression(&node.consequent, depth + 2);
                self.push_line(depth + 1, "Alternate:");
                self.write_expression(&node.alternate, depth + 2);
            }
            gc3::Expression::Sequence(node) => {
                self.push_node(
                    depth,
                    "SequenceExpression",
                    node.span,
                    Some(format!("expressions={}", node.expressions.len())),
                    false,
                );
                for expression in &node.expressions {
                    self.write_expression(expression, depth + 1);
                }
            }
            gc3::Expression::Call(node) => {
                self.push_node(
                    depth,
                    "CallExpression",
                    node.span,
                    Some(format!(
                        "args={} optional={}",
                        node.arguments.len(),
                        node.optional
                    )),
                    false,
                );
                self.push_line(depth + 1, "Callee:");
                self.write_expression(&node.callee, depth + 2);
                for argument in &node.arguments {
                    self.write_call_argument(argument, depth + 1);
                }
            }
            gc3::Expression::Member(node) => {
                self.push_node(
                    depth,
                    "MemberExpression",
                    node.span,
                    Some(format!(
                        "property={} optional={}",
                        member_property_summary(&node.property),
                        node.optional
                    )),
                    false,
                );
                self.push_line(depth + 1, "Object:");
                self.write_expression(&node.object, depth + 2);
                if let gc3::MemberProperty::Computed { expression, .. } = &node.property {
                    self.push_line(depth + 1, "ComputedProperty:");
                    self.write_expression(expression, depth + 2);
                }
            }
            gc3::Expression::New(node) => {
                self.push_node(
                    depth,
                    "NewExpression",
                    node.span,
                    Some(format!("args={}", node.arguments.len())),
                    false,
                );
                self.push_line(depth + 1, "Callee:");
                self.write_expression(&node.callee, depth + 2);
                for argument in &node.arguments {
                    self.write_call_argument(argument, depth + 1);
                }
            }
        }
    }

    fn write_call_argument(&mut self, argument: &gc3::CallArgument, depth: usize) {
        match argument {
            gc3::CallArgument::Expression(expression) => {
                self.push_line(depth, "Argument:");
                self.write_expression(expression, depth + 1);
            }
            gc3::CallArgument::Spread { argument, span } => {
                self.push_node(depth, "SpreadArgument", *span, None, false);
                self.write_expression(argument, depth + 1);
            }
        }
    }

    fn write_object_property(&mut self, property: &gc3::ObjectProperty, depth: usize) {
        match property {
            gc3::ObjectProperty::Property {
                key,
                value,
                shorthand,
                kind,
                span,
            } => {
                self.push_node(
                    depth,
                    "ObjectProperty",
                    *span,
                    Some(format!(
                        "key={} shorthand={} kind={:?}",
                        property_key_summary(key),
                        shorthand,
                        kind
                    )),
                    false,
                );
                self.write_expression(value, depth + 1);
            }
            gc3::ObjectProperty::Spread { argument, span } => {
                self.push_node(depth, "ObjectSpread", *span, None, false);
                self.write_expression(argument, depth + 1);
            }
        }
    }

    fn write_literal(&mut self, literal: &gc3::Literal, depth: usize) {
        self.push_node(
            depth,
            "Literal",
            literal.span(),
            Some(literal_summary(literal)),
            false,
        );
    }
}

fn import_clause_summary(declaration: &gc3::ImportDeclaration) -> String {
    let clause = match &declaration.clause {
        None => "side-effect".to_owned(),
        Some(gc3::ImportClause::Default(identifier)) => format!("default={}", identifier.name),
        Some(gc3::ImportClause::Namespace { default, namespace }) => match default {
            Some(default) => format!("default={} namespace={}", default.name, namespace.name),
            None => format!("namespace={}", namespace.name),
        },
        Some(gc3::ImportClause::Named {
            default,
            specifiers,
        }) => {
            let default = default
                .as_ref()
                .map(|identifier| format!("default={} ", identifier.name))
                .unwrap_or_default();
            format!("{default}named={}", specifiers.len())
        }
    };
    format!("source={:?} {clause}", declaration.source.value)
}

fn module_export_name_summary(name: &gc3::ModuleExportName) -> String {
    match name {
        gc3::ModuleExportName::Identifier(identifier) => identifier.name.clone(),
        gc3::ModuleExportName::String(string) => format!("{:?}", string.value),
    }
}

fn property_key_summary(key: &gc3::PropertyKey) -> String {
    match key {
        gc3::PropertyKey::Identifier(identifier) => identifier.name.clone(),
        gc3::PropertyKey::PrivateName(identifier) => format!("#{}", identifier.name),
        gc3::PropertyKey::String(string) => format!("{:?}", string.value),
        gc3::PropertyKey::Number(number) => number.raw.clone(),
        gc3::PropertyKey::Computed { expression, .. } => {
            format!("[{}]", summarize_expression(expression))
        }
    }
}

fn member_property_summary(property: &gc3::MemberProperty) -> String {
    match property {
        gc3::MemberProperty::Identifier(identifier) => identifier.name.clone(),
        gc3::MemberProperty::PrivateName(identifier) => format!("#{}", identifier.name),
        gc3::MemberProperty::Computed { expression, .. } => {
            format!("[{}]", summarize_expression(expression))
        }
    }
}

fn variable_kind_name(kind: gc3::VariableKind) -> &'static str {
    match kind {
        gc3::VariableKind::Var => "var",
        gc3::VariableKind::Let => "let",
        gc3::VariableKind::Const => "const",
        gc3::VariableKind::Using => "using",
        gc3::VariableKind::AwaitUsing => "await using",
    }
}

fn literal_summary(literal: &gc3::Literal) -> String {
    match literal {
        gc3::Literal::Null(_) => "kind=null".to_owned(),
        gc3::Literal::Boolean(node) => format!("kind=boolean value={}", node.value),
        gc3::Literal::Number(node) => format!("kind=number raw={}", node.raw),
        gc3::Literal::String(node) => {
            format!(
                "kind=string value={:?}",
                truncate_text(&node.value, MAX_SUMMARY_CHARS)
            )
        }
        gc3::Literal::Template(node) => {
            format!(
                "kind=template value={:?}",
                truncate_text(&node.value, MAX_SUMMARY_CHARS)
            )
        }
        gc3::Literal::RegExp(node) => format!("kind=regexp /{}/{}", node.body, node.flags),
    }
}

fn summarize_expression(expression: &gc3::Expression) -> String {
    truncate_text(&expression_to_js(expression), MAX_SUMMARY_CHARS)
}

fn format_span(span: gc3::Span) -> String {
    format!(
        "{}:{}-{}:{}",
        span.start.line, span.start.column, span.end.line, span.end.column
    )
}

pub fn snippet_for_span(source: &str, span: gc3::Span) -> Option<String> {
    let start = span.start.offset.min(source.len());
    let end = span.end.offset.min(source.len());
    if start >= end {
        return None;
    }
    let snippet = &source[start..end];
    let collapsed = collapse_whitespace(snippet);
    (!collapsed.is_empty()).then(|| truncate_text(&collapsed, MAX_SNIPPET_CHARS))
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }

    let mut truncated = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= max_chars.saturating_sub(3) {
            break;
        }
        truncated.push(ch);
    }
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::{
        AstPrintMode, AstPrinter, ReplOptions, build_eval_reports, format_enabled_options,
        parse_startup_config, snippet_for_span,
    };
    use vm::codegen::compile_source;

    #[test]
    fn parses_startup_flags_and_load_command() {
        let args = vec![
            "--ast".to_owned(),
            "--disasm".to_owned(),
            "--opt".to_owned(),
            ".load".to_owned(),
            "fib.qjs".to_owned(),
        ];
        let config = parse_startup_config(&args).expect("startup config should parse");
        assert_eq!(
            config.repl_options,
            ReplOptions {
                show_ast: true,
                show_disasm: true,
                optimize: true,
            }
        );
        assert_eq!(config.command_args, vec![".load", "fib.qjs"]);
    }

    #[test]
    fn parses_one_shot_aliases() {
        let once = parse_startup_config(&["--once".to_owned(), "fib.qjs".to_owned()])
            .expect("`--once` should parse");
        assert!(once.exit_after_startup);
        assert_eq!(once.command_args, vec!["fib.qjs"]);

        let batch = parse_startup_config(&["--batch".to_owned(), ".load".to_owned()])
            .expect("`--batch` should parse");
        assert!(batch.exit_after_startup);
        assert_eq!(batch.command_args, vec![".load"]);
    }

    #[test]
    fn parses_repl_options_with_one_shot_mode() {
        let config = parse_startup_config(&[
            "--ast".to_owned(),
            "--disasm".to_owned(),
            "--opt".to_owned(),
            "--once".to_owned(),
            "fib.qjs".to_owned(),
        ])
        .expect("options should parse");

        assert_eq!(
            config.repl_options,
            ReplOptions {
                show_ast: true,
                show_disasm: true,
                optimize: true,
            }
        );
        assert!(config.exit_after_startup);
        assert_eq!(config.command_args, vec!["fib.qjs"]);
    }

    #[test]
    fn enabled_options_are_rendered_in_order() {
        let rendered = format_enabled_options(ReplOptions {
            show_ast: true,
            show_disasm: true,
            optimize: true,
        });
        assert_eq!(rendered.as_deref(), Some("--ast, --disasm, --opt"));
    }

    #[test]
    fn source_snippets_collapse_whitespace() {
        let source = "if (\n  foo\n) {\n  bar();\n}";
        let program = gc3::parse(source).expect("source should parse");
        let span = program.body[0].span();
        let snippet = snippet_for_span(source, span).expect("snippet should exist");
        assert_eq!(snippet, "if ( foo ) { bar(); }");
    }

    #[test]
    fn full_ast_printer_includes_expression_nodes() {
        let source = "let x = 1 + 2;";
        let program = gc3::parse(source).expect("source should parse");
        let lines = AstPrinter::new(source, AstPrintMode::Full).render_program(&program);
        assert!(lines.iter().any(|line| line.contains("BinaryExpression")));
    }

    #[test]
    fn disasm_report_omits_source_outline() {
        let source = "let x = 1 + 2;";
        let program = gc3::parse(source).expect("source should parse");
        let compiled = compile_source(source).expect("source should compile");
        let reports = build_eval_reports(
            source,
            &program,
            &compiled,
            ReplOptions {
                show_ast: false,
                show_disasm: true,
                optimize: false,
            },
        );

        assert_eq!(
            reports.disasm_lines.get(0).map(std::string::String::as_str),
            Some("== Disasm ==")
        );
        assert_eq!(
            reports.disasm_lines.get(1).map(std::string::String::as_str),
            Some("Bytecode:")
        );
        assert!(
            !reports
                .disasm_lines
                .iter()
                .any(|line| line.contains("Source outline:") || line.starts_with("Program @"))
        );
    }

    #[test]
    fn outline_printer_omits_expression_nodes() {
        let source = "let x = 1 + 2;";
        let program = gc3::parse(source).expect("source should parse");
        let lines = AstPrinter::new(source, AstPrintMode::Outline).render_program(&program);
        assert!(!lines.iter().any(|line| line.contains("BinaryExpression")));
    }
}
