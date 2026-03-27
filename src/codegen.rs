use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt;

use gc3::{
    ArrowBody, ArrowFunction, AssignmentExpression, AssignmentOperator, BinaryExpression,
    BinaryOperator, BlockStatement, CallArgument, CallExpression, ConditionalExpression,
    DoWhileStatement, Expression, ExpressionStatement, ForClassicStatement, ForInit, ForStatement,
    Function, Identifier, IfStatement, Literal, LogicalExpression, LogicalOperator,
    MemberExpression, MemberProperty, NewExpression, NumberLiteral, ObjectExpression,
    ObjectProperty, ObjectPropertyKind, Pattern, Program, PropertyKey, ReturnStatement,
    SequenceExpression, Span, Statement, StringLiteral, UnaryExpression, UnaryOperator,
    UpdateExpression, UpdateOperator, VariableDeclaration, VariableDeclarator, WhileStatement,
};

use crate::emit::BytecodeBuilder;
use crate::js_value::{JSValue, make_number, make_undefined};
use crate::vm::Opcode;

const ACC: u8 = 255;
const MAX_TEMP_REG: u8 = ACC - 1;

#[derive(Debug, Clone)]
pub struct CompiledBytecode {
    pub bytecode: Vec<u32>,
    pub constants: Vec<JSValue>,
    pub string_constants: Vec<(u16, String)>,
    pub function_constants: Vec<u16>,
    pub names: Vec<String>,
    pub properties: Vec<String>,
}

#[derive(Debug)]
pub enum CodegenError {
    Parse(gc3::ParseError),
    Unsupported { feature: &'static str, span: Span },
    RegisterOverflow { span: Option<Span> },
    NameOverflow { name: String },
    PropertyOverflow { name: String },
    NumericLiteral { raw: String, span: Span },
    InvalidBreak { span: Span },
    InvalidContinue { span: Span },
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::Unsupported { feature, .. } => write!(f, "unsupported AST feature: {feature}"),
            Self::RegisterOverflow { .. } => write!(f, "temporary register overflow"),
            Self::NameOverflow { name } => {
                write!(f, "too many bound identifiers, cannot encode `{name}`")
            }
            Self::PropertyOverflow { name } => {
                write!(f, "too many property names, cannot encode `{name}`")
            }
            Self::NumericLiteral { raw, .. } => write!(f, "invalid numeric literal `{raw}`"),
            Self::InvalidBreak { .. } => write!(f, "`break` used outside of a loop"),
            Self::InvalidContinue { .. } => write!(f, "`continue` used outside of a loop"),
        }
    }
}

impl Error for CodegenError {}

impl From<gc3::ParseError> for CodegenError {
    fn from(value: gc3::ParseError) -> Self {
        Self::Parse(value)
    }
}

#[derive(Debug, Clone)]
enum PendingFunctionBody {
    Function(Function),
    Arrow(ArrowFunction),
}

#[derive(Debug, Clone)]
struct PendingFunction {
    const_index: u16,
    body: PendingFunctionBody,
}

#[derive(Debug, Clone, Copy)]
enum JumpPatchKind {
    Jmp,
    JmpFalse { reg: u8 },
    JmpLteFalse { lhs: u8, rhs: u8 },
    Try,
}

#[derive(Debug, Clone, Copy)]
struct JumpPatch {
    pos: usize,
    target: usize,
    kind: JumpPatchKind,
}

#[derive(Debug, Default)]
struct ControlContext {
    break_patches: Vec<usize>,
    continue_patches: Option<Vec<usize>>,
}

impl ControlContext {
    fn loop_context() -> Self {
        Self {
            break_patches: Vec::new(),
            continue_patches: Some(Vec::new()),
        }
    }

    fn switch_context() -> Self {
        Self {
            break_patches: Vec::new(),
            continue_patches: None,
        }
    }
}

pub fn compile_source(source: &str) -> Result<CompiledBytecode, CodegenError> {
    let program = gc3::parse(source)?;
    compile_program(&program)
}

pub fn compile_program(program: &Program) -> Result<CompiledBytecode, CodegenError> {
    let mut codegen = Codegen::new();
    let last_value = codegen.compile_statement_list(&program.body, true)?;
    codegen.finish_root(last_value);
    codegen.compile_pending_functions()?;
    codegen.finalize()
}

struct Codegen {
    builder: BytecodeBuilder,
    name_slots: HashMap<String, u16>,
    property_slots: HashMap<String, u8>,
    fast_name_regs: HashMap<String, u8>,
    fast_name_scope_stack: Vec<Vec<(String, Option<u8>)>>,
    names: Vec<String>,
    properties: Vec<String>,
    string_constants: Vec<(u16, String)>,
    undefined_const: Option<u16>,
    jump_patches: Vec<JumpPatch>,
    function_patches: Vec<(u16, usize)>,
    pending_functions: VecDeque<PendingFunction>,
    temp_top: u8,
    control_stack: Vec<ControlContext>,
    nested_scope_depth: usize,
    fast_name_bindings_enabled: bool,
}

impl Codegen {
    fn new() -> Self {
        Self {
            builder: BytecodeBuilder::new(),
            name_slots: HashMap::new(),
            property_slots: HashMap::new(),
            fast_name_regs: HashMap::new(),
            fast_name_scope_stack: Vec::new(),
            names: Vec::new(),
            properties: Vec::new(),
            string_constants: Vec::new(),
            undefined_const: None,
            jump_patches: Vec::new(),
            function_patches: Vec::new(),
            pending_functions: VecDeque::new(),
            temp_top: 0,
            control_stack: Vec::new(),
            nested_scope_depth: 0,
            fast_name_bindings_enabled: false,
        }
    }

    fn finalize(self) -> Result<CompiledBytecode, CodegenError> {
        let (mut bytecode, mut constants) = self.builder.build();

        for patch in self.jump_patches {
            let offset = patch.target as isize - patch.pos as isize - 1;
            let offset = i16::try_from(offset).map_err(|_| CodegenError::Unsupported {
                feature: "jump offset out of range",
                span: Span::default(),
            })?;
            bytecode[patch.pos] = match patch.kind {
                JumpPatchKind::Jmp => encode_asbx(Opcode::Jmp, 0, offset),
                JumpPatchKind::JmpFalse { reg } => encode_asbx(Opcode::JmpFalse, reg, offset),
                JumpPatchKind::JmpLteFalse { lhs, rhs } => {
                    let short = i8::try_from(offset).map_err(|_| CodegenError::Unsupported {
                        feature: "short compare jump offset out of range",
                        span: Span::default(),
                    })?;
                    ((short as u8 as u32) << 24)
                        | ((rhs as u32) << 16)
                        | ((lhs as u32) << 8)
                        | Opcode::JmpLteFalse.as_u8() as u32
                }
                JumpPatchKind::Try => encode_asbx(Opcode::Try, 0, offset),
            };
        }

        let function_constants = self
            .function_patches
            .iter()
            .map(|(const_index, _)| *const_index)
            .collect::<Vec<_>>();

        for (const_index, entry_pc) in self.function_patches {
            if let Some(slot) = constants.get_mut(const_index as usize) {
                *slot = make_number(entry_pc as f64);
            }
        }

        Ok(CompiledBytecode {
            bytecode,
            constants,
            string_constants: self.string_constants,
            function_constants,
            names: self.names,
            properties: self.properties,
        })
    }

    fn compile_pending_functions(&mut self) -> Result<(), CodegenError> {
        while let Some(pending) = self.pending_functions.pop_front() {
            let entry_pc = self.builder.len();
            self.function_patches.push((pending.const_index, entry_pc));

            let saved_temp_top = self.temp_top;
            let saved_fast_name_regs = self.fast_name_regs.clone();
            let saved_fast_name_scope_stack = self.fast_name_scope_stack.clone();
            let saved_nested_scope_depth = self.nested_scope_depth;
            let saved_fast_name_bindings_enabled = self.fast_name_bindings_enabled;
            self.temp_top = 0;
            self.fast_name_regs.clear();
            self.fast_name_scope_stack.clear();
            self.nested_scope_depth = 0;
            self.fast_name_bindings_enabled = true;
            self.enter_fast_name_scope();

            let env_reg = self.alloc_temp(None)?;
            self.builder.emit_create_env(env_reg);

            match pending.body {
                PendingFunctionBody::Function(function) => {
                    self.compile_function_params(&function.params)?;
                    self.compile_statement_list(&function.body.body, false)?;
                    self.builder.emit_ret_u();
                }
                PendingFunctionBody::Arrow(function) => {
                    self.compile_function_params(&function.params)?;
                    match function.body {
                        ArrowBody::Expression(expression) => {
                            let reg = self.compile_expression(&expression)?;
                            self.builder.emit_mov(ACC, reg);
                            self.temp_top = reg;
                            self.builder.emit_ret();
                        }
                        ArrowBody::Block(block) => {
                            self.compile_statement_list(&block.body, false)?;
                            self.builder.emit_ret_u();
                        }
                    }
                }
            }

            self.leave_fast_name_scope();
            self.temp_top = saved_temp_top;
            self.fast_name_regs = saved_fast_name_regs;
            self.fast_name_scope_stack = saved_fast_name_scope_stack;
            self.nested_scope_depth = saved_nested_scope_depth;
            self.fast_name_bindings_enabled = saved_fast_name_bindings_enabled;
        }

        Ok(())
    }

    fn finish_root(&mut self, last_value: Option<u8>) {
        if let Some(reg) = last_value {
            self.builder.emit_mov(ACC, reg);
            self.builder.emit_ret();
        } else {
            self.builder.emit_ret_u();
        }
    }

    fn compile_statement_list(
        &mut self,
        statements: &[Statement],
        root_script: bool,
    ) -> Result<Option<u8>, CodegenError> {
        let mut last_value = None;
        for statement in statements {
            let value = self.compile_statement(statement, root_script)?;
            if value.is_some() {
                last_value = value;
            }
        }
        Ok(last_value)
    }

    fn compile_statement(
        &mut self,
        statement: &Statement,
        root_script: bool,
    ) -> Result<Option<u8>, CodegenError> {
        match statement {
            Statement::Directive(_) | Statement::Empty(_) | Statement::Debugger(_) => Ok(None),
            Statement::Block(block) => self.compile_block(block, !root_script),
            Statement::VariableDeclaration(declaration) => {
                self.compile_variable_declaration(declaration)?;
                Ok(None)
            }
            Statement::FunctionDeclaration(function) => {
                self.compile_function_declaration(function)?;
                Ok(None)
            }
            Statement::If(statement) => {
                self.compile_if_statement(statement, root_script)?;
                Ok(None)
            }
            Statement::While(statement) => {
                self.compile_while_statement(statement, root_script)?;
                Ok(None)
            }
            Statement::DoWhile(statement) => {
                self.compile_do_while_statement(statement, root_script)?;
                Ok(None)
            }
            Statement::For(statement) => {
                self.compile_for_statement(statement, root_script)?;
                Ok(None)
            }
            Statement::Return(statement) => {
                self.compile_return_statement(statement)?;
                Ok(None)
            }
            Statement::Break(jump) => {
                if jump.label.is_some() {
                    return Err(CodegenError::Unsupported {
                        feature: "labeled break",
                        span: jump.span,
                    });
                }
                let patch = self.emit_placeholder_jmp();
                let Some(control_ctx) = self.control_stack.last_mut() else {
                    return Err(CodegenError::InvalidBreak { span: jump.span });
                };
                control_ctx.break_patches.push(patch);
                Ok(None)
            }
            Statement::Continue(jump) => {
                if jump.label.is_some() {
                    return Err(CodegenError::Unsupported {
                        feature: "labeled continue",
                        span: jump.span,
                    });
                }
                let patch = self.emit_placeholder_jmp();
                let Some(control_ctx) = self
                    .control_stack
                    .iter_mut()
                    .rev()
                    .find(|ctx| ctx.continue_patches.is_some())
                else {
                    return Err(CodegenError::InvalidContinue { span: jump.span });
                };
                control_ctx
                    .continue_patches
                    .as_mut()
                    .expect("loop continue patches")
                    .push(patch);
                Ok(None)
            }
            Statement::Expression(ExpressionStatement { expression, .. }) => {
                self.compile_expression(expression).map(Some)
            }
            Statement::Labeled(node) => Err(CodegenError::Unsupported {
                feature: "labeled statements",
                span: node.span,
            }),
            Statement::ImportDeclaration(node) => Err(CodegenError::Unsupported {
                feature: "imports",
                span: node.span,
            }),
            Statement::ExportDeclaration(node) => Err(CodegenError::Unsupported {
                feature: "exports",
                span: node.span(),
            }),
            Statement::ClassDeclaration(node) => Err(CodegenError::Unsupported {
                feature: "classes",
                span: node.span,
            }),
            Statement::Switch(node) => {
                self.compile_switch_statement(node, root_script)?;
                Ok(None)
            }
            Statement::Throw(node) => {
                self.compile_throw_statement(node)?;
                Ok(None)
            }
            Statement::Try(node) => {
                self.compile_try_statement(node)?;
                Ok(None)
            }
            Statement::With(node) => Err(CodegenError::Unsupported {
                feature: "with",
                span: node.span,
            }),
        }
    }

    fn compile_block(
        &mut self,
        block: &BlockStatement,
        create_scope: bool,
    ) -> Result<Option<u8>, CodegenError> {
        if !create_scope {
            return self.compile_statement_list(&block.body, false);
        }

        let saved_top = self.temp_top;
        self.nested_scope_depth += 1;
        self.enter_fast_name_scope();
        self.builder.emit_enter(256);
        let env_reg = self.alloc_temp(Some(block.span))?;
        self.builder.emit_create_env(env_reg);
        let last = self.compile_statement_list(&block.body, false)?;
        self.builder.emit_leave();
        self.leave_fast_name_scope();
        self.nested_scope_depth = self.nested_scope_depth.saturating_sub(1);
        self.temp_top = saved_top;
        Ok(last)
    }

    fn compile_if_statement(
        &mut self,
        statement: &IfStatement,
        root_script: bool,
    ) -> Result<(), CodegenError> {
        if self.compile_fused_if_return(statement)? {
            return Ok(());
        }

        let (false_jump, false_kind, test_top) = self.compile_condition_jump_false(&statement.test)?;
        self.compile_statement(&statement.consequent, root_script)?;

        if let Some(alternate) = &statement.alternate {
            let end_jump = self.emit_placeholder_jmp();
            let alternate_start = self.builder.len();
            self.patch_jump(false_jump, alternate_start, false_kind);
            self.compile_statement(alternate, root_script)?;
            let end = self.builder.len();
            self.patch_jump(end_jump, end, JumpPatchKind::Jmp);
        } else {
            let end = self.builder.len();
            self.patch_jump(false_jump, end, false_kind);
        }

        self.temp_top = test_top.saturating_sub(1);
        Ok(())
    }

    fn compile_fused_if_return(&mut self, statement: &IfStatement) -> Result<bool, CodegenError> {
        if statement.alternate.is_some() {
            return Ok(false);
        }

        let Statement::Return(return_stmt) = statement.consequent.as_ref() else {
            return Ok(false);
        };
        let Some(Expression::Identifier(return_id)) = return_stmt.argument.as_ref() else {
            return Ok(false);
        };
        let Expression::Binary(binary) = &statement.test else {
            return Ok(false);
        };
        if binary.operator != BinaryOperator::LessThanOrEqual {
            return Ok(false);
        }
        let Expression::Identifier(lhs_id) = &binary.left else {
            return Ok(false);
        };
        if lhs_id.name != return_id.name {
            return Ok(false);
        }

        let lhs = self.compile_identifier_current(lhs_id)?;
        let rhs = self.compile_readonly_expression(&binary.right)?;
        self.builder.emit_ret_if_lte_i(lhs, rhs, lhs);
        self.temp_top = lhs.max(rhs).saturating_sub(1);
        Ok(true)
    }

    fn compile_while_statement(
        &mut self,
        statement: &WhileStatement,
        root_script: bool,
    ) -> Result<(), CodegenError> {
        let loop_start = self.builder.len();
        let (exit_jump, exit_kind, test_top) = self.compile_condition_jump_false(&statement.test)?;

        self.control_stack.push(ControlContext::loop_context());
        self.compile_statement(&statement.body, root_script)?;
        let loop_ctx = self.control_stack.pop().expect("loop context");
        self.patch_jump(exit_jump, self.builder.len(), exit_kind);

        let continue_target = loop_start;
        self.patch_loop_continues(
            loop_ctx.continue_patches.expect("loop continue patches"),
            continue_target,
        );

        self.builder
            .emit_jmp(offset_to(loop_start, self.builder.len())?);

        let end = self.builder.len();
        self.patch_loop_breaks(loop_ctx.break_patches, end);
        self.temp_top = test_top.saturating_sub(1);
        Ok(())
    }

    fn compile_do_while_statement(
        &mut self,
        statement: &DoWhileStatement,
        root_script: bool,
    ) -> Result<(), CodegenError> {
        let body_start = self.builder.len();
        self.control_stack.push(ControlContext::loop_context());
        self.compile_statement(&statement.body, root_script)?;
        let test_start = self.builder.len();
        let test_reg = self.compile_expression(&statement.test)?;
        self.builder
            .emit_jmp_true(test_reg, offset_to(body_start, self.builder.len())?);

        let loop_ctx = self.control_stack.pop().expect("loop context");
        self.patch_loop_continues(
            loop_ctx.continue_patches.expect("loop continue patches"),
            test_start,
        );
        let end = self.builder.len();
        self.patch_loop_breaks(loop_ctx.break_patches, end);
        self.temp_top = test_reg.saturating_sub(1);
        Ok(())
    }

    fn compile_for_statement(
        &mut self,
        statement: &ForStatement,
        root_script: bool,
    ) -> Result<(), CodegenError> {
        match statement {
            ForStatement::Classic(classic) => {
                self.compile_for_classic_statement(classic, root_script)
            }
            ForStatement::In(node) => Err(CodegenError::Unsupported {
                feature: "for-in",
                span: node.span,
            }),
            ForStatement::Of(node) => self.compile_for_of_statement(node, root_script),
        }
    }

    fn compile_for_classic_statement(
        &mut self,
        statement: &ForClassicStatement,
        root_script: bool,
    ) -> Result<(), CodegenError> {
        self.nested_scope_depth += 1;
        self.enter_fast_name_scope();
        self.builder.emit_enter(256);
        let env_reg = self.alloc_temp(Some(statement.span))?;
        self.builder.emit_create_env(env_reg);

        if let Some(init) = &statement.init {
            match init {
                ForInit::VariableDeclaration(declaration) => {
                    self.compile_variable_declaration(declaration)?;
                }
                ForInit::Expression(expression) => {
                    let reg = self.compile_expression(expression)?;
                    self.temp_top = reg.saturating_sub(1);
                }
            }
        }

        let loop_start = self.builder.len();
        let exit_jump = if let Some(test) = &statement.test {
            let (patch, kind, top) = self.compile_condition_jump_false(test)?;
            Some((patch, kind, top))
        } else {
            None
        };

        self.control_stack.push(ControlContext::loop_context());
        self.compile_statement(&statement.body, root_script)?;
        let mut loop_ctx = self.control_stack.pop().expect("loop context");

        let continue_target = self.builder.len();
        self.patch_loop_continues(
            std::mem::take(
                loop_ctx
                    .continue_patches
                    .as_mut()
                    .expect("loop continue patches"),
            ),
            continue_target,
        );

        if let Some(update) = &statement.update {
            let reg = self.compile_expression(update)?;
            self.temp_top = reg.saturating_sub(1);
        }

        self.builder
            .emit_jmp(offset_to(loop_start, self.builder.len())?);

        let end = self.builder.len();
        if let Some((patch_pos, kind, test_top)) = exit_jump {
            self.patch_jump(patch_pos, end, kind);
            self.temp_top = test_top.saturating_sub(1);
        }
        self.patch_loop_breaks(loop_ctx.break_patches, end);
        self.builder.emit_leave();
        self.leave_fast_name_scope();
        self.nested_scope_depth = self.nested_scope_depth.saturating_sub(1);
        self.temp_top = env_reg.saturating_sub(1);
        Ok(())
    }

    fn compile_for_of_statement(
        &mut self,
        statement: &gc3::ForEachStatement,
        root_script: bool,
    ) -> Result<(), CodegenError> {
        if statement.is_await {
            return Err(CodegenError::Unsupported {
                feature: "for-await-of",
                span: statement.span,
            });
        }

        self.nested_scope_depth += 1;
        self.enter_fast_name_scope();
        self.builder.emit_enter(256);
        let env_reg = self.alloc_temp(Some(statement.span))?;
        self.builder.emit_create_env(env_reg);

        let iterable_reg = self.compile_expression(&statement.right)?;
        let index_reg = self.alloc_temp(Some(statement.span))?;
        self.builder.emit_load_i(index_reg, 0);

        let loop_start = self.builder.len();
        let length_reg = self.alloc_temp(Some(statement.span))?;
        self.builder.emit_get_length_ic(length_reg, iterable_reg, 0);
        self.builder.emit_lt(index_reg, length_reg);
        let cond_reg = self.alloc_temp(Some(statement.span))?;
        self.builder.emit_mov(cond_reg, ACC);
        let exit_jump = self.emit_placeholder_jmp_false(cond_reg);

        self.builder.emit_get_prop_acc(iterable_reg, index_reg);
        let value_reg = self.alloc_temp(Some(statement.span))?;
        self.builder.emit_mov(value_reg, ACC);
        self.bind_for_each_left(&statement.left, value_reg, statement.span)?;

        self.control_stack.push(ControlContext::loop_context());
        self.compile_statement(&statement.body, root_script)?;
        let mut loop_ctx = self.control_stack.pop().expect("loop context");

        let continue_target = self.builder.len();
        self.patch_loop_continues(
            std::mem::take(
                loop_ctx
                    .continue_patches
                    .as_mut()
                    .expect("loop continue patches"),
            ),
            continue_target,
        );

        self.builder.emit_inc(index_reg);
        self.builder.emit_mov(index_reg, ACC);
        self.builder
            .emit_jmp(offset_to(loop_start, self.builder.len())?);

        let end = self.builder.len();
        self.patch_jump(exit_jump, end, JumpPatchKind::JmpFalse { reg: cond_reg });
        self.patch_loop_breaks(loop_ctx.break_patches, end);
        self.builder.emit_leave();
        self.leave_fast_name_scope();
        self.nested_scope_depth = self.nested_scope_depth.saturating_sub(1);
        self.temp_top = env_reg.saturating_sub(1);
        Ok(())
    }

    fn compile_switch_statement(
        &mut self,
        statement: &gc3::SwitchStatement,
        _root_script: bool,
    ) -> Result<(), CodegenError> {
        let discriminant_reg = self.compile_expression(&statement.discriminant)?;
        let mut pending_false_jump: Option<(usize, u8)> = None;
        let mut case_entry_jumps = Vec::new();
        let mut default_case = None;

        for (index, case) in statement.cases.iter().enumerate() {
            if let Some((patch, reg)) = pending_false_jump.take() {
                self.patch_jump(patch, self.builder.len(), JumpPatchKind::JmpFalse { reg });
            }

            if let Some(test) = &case.test {
                let case_reg = self.compile_expression(test)?;
                self.builder.emit_strict_eq(discriminant_reg, case_reg);
                let cond_reg = self.alloc_temp(Some(case.span))?;
                self.builder.emit_mov(cond_reg, ACC);
                let false_jump = self.emit_placeholder_jmp_false(cond_reg);
                let matched_jump = self.emit_placeholder_jmp();
                pending_false_jump = Some((false_jump, cond_reg));
                case_entry_jumps.push((index, matched_jump));
                self.temp_top = discriminant_reg;
            } else {
                default_case = Some(index);
            }
        }

        let default_dispatch = self.builder.len();
        let default_jump = self.emit_placeholder_jmp();
        if let Some((patch, reg)) = pending_false_jump.take() {
            self.patch_jump(patch, default_dispatch, JumpPatchKind::JmpFalse { reg });
        }

        self.control_stack.push(ControlContext::switch_context());
        let mut case_starts = vec![None; statement.cases.len()];
        for (index, case) in statement.cases.iter().enumerate() {
            case_starts[index] = Some(self.builder.len());
            self.compile_statement_list(&case.consequent, false)?;
        }
        let switch_ctx = self.control_stack.pop().expect("switch context");
        let end = self.builder.len();
        self.patch_loop_breaks(switch_ctx.break_patches, end);

        for (index, jump_pos) in case_entry_jumps {
            let target = case_starts[index].unwrap_or(end);
            self.patch_jump(jump_pos, target, JumpPatchKind::Jmp);
        }
        let default_target = default_case
            .and_then(|index| case_starts[index])
            .unwrap_or(end);
        self.patch_jump(default_jump, default_target, JumpPatchKind::Jmp);

        self.temp_top = discriminant_reg.saturating_sub(1);
        Ok(())
    }

    fn compile_return_statement(
        &mut self,
        statement: &ReturnStatement,
    ) -> Result<(), CodegenError> {
        if let Some(argument) = &statement.argument {
            let reg = match argument {
                Expression::Identifier(identifier) => self.compile_identifier_current(identifier)?,
                _ => self.compile_expression(argument)?,
            };
            self.temp_top = reg.saturating_sub(1);
            self.builder.emit_ret_reg(reg);
        } else {
            self.builder.emit_ret_u();
        }
        Ok(())
    }

    fn compile_throw_statement(
        &mut self,
        statement: &gc3::ThrowStatement,
    ) -> Result<(), CodegenError> {
        let reg = self.compile_expression(&statement.argument)?;
        self.builder.emit_throw(reg);
        self.temp_top = reg.saturating_sub(1);
        Ok(())
    }

    fn compile_try_statement(&mut self, statement: &gc3::TryStatement) -> Result<(), CodegenError> {
        if statement.finalizer.is_some() {
            return Err(CodegenError::Unsupported {
                feature: "try/finally",
                span: statement.span,
            });
        }

        let Some(handler) = &statement.handler else {
            return Err(CodegenError::Unsupported {
                feature: "try without catch",
                span: statement.span,
            });
        };

        let try_patch = self.builder.len();
        self.builder.emit_try(0);
        self.compile_statement_list(&statement.block.body, false)?;
        self.builder.emit_end_try();
        let skip_catch = self.emit_placeholder_jmp();

        let catch_start = self.builder.len();
        self.patch_jump(try_patch, catch_start, JumpPatchKind::Try);
        self.compile_catch_clause(handler)?;

        let end = self.builder.len();
        self.patch_jump(skip_catch, end, JumpPatchKind::Jmp);
        Ok(())
    }

    fn compile_catch_clause(&mut self, clause: &gc3::CatchClause) -> Result<(), CodegenError> {
        let saved_top = self.temp_top;
        self.nested_scope_depth += 1;
        self.enter_fast_name_scope();
        self.builder.emit_enter(256);
        let env_reg = self.alloc_temp(Some(clause.span))?;
        self.builder.emit_create_env(env_reg);
        let exception_reg = self.alloc_temp(Some(clause.span))?;
        self.builder.emit_catch(exception_reg);

        if let Some(param) = &clause.param {
            match param {
                Pattern::Identifier(identifier) => {
                    let slot = self.name_slot(&identifier.name)?;
                    self.builder.emit_init_name(exception_reg, slot);
                    self.promote_fast_name(&identifier.name, exception_reg);
                }
                other => {
                    return Err(CodegenError::Unsupported {
                        feature: "complex catch parameter",
                        span: other.span(),
                    });
                }
            }
        }

        self.compile_statement_list(&clause.body.body, false)?;
        self.builder.emit_leave();
        self.builder.emit_finally();
        self.leave_fast_name_scope();
        self.nested_scope_depth = self.nested_scope_depth.saturating_sub(1);
        self.temp_top = saved_top;
        Ok(())
    }

    fn bind_for_each_left(
        &mut self,
        left: &gc3::ForLeft,
        value_reg: u8,
        span: Span,
    ) -> Result<(), CodegenError> {
        match left {
            gc3::ForLeft::VariableDeclaration(declaration) => {
                if declaration.declarations.len() != 1 {
                    return Err(CodegenError::Unsupported {
                        feature: "multiple for-of bindings",
                        span: declaration.span,
                    });
                }
                let declarator = &declaration.declarations[0];
                if declarator.init.is_some() {
                    return Err(CodegenError::Unsupported {
                        feature: "initialized for-of bindings",
                        span: declarator.span,
                    });
                }
                self.bind_pattern_value(&declarator.pattern, value_reg, span, true)
            }
            gc3::ForLeft::Pattern(pattern) => {
                self.bind_pattern_value(pattern, value_reg, span, false)
            }
            gc3::ForLeft::Expression(expression) => {
                self.bind_assignment_target(expression, value_reg)
            }
        }
    }

    fn bind_pattern_value(
        &mut self,
        pattern: &Pattern,
        value_reg: u8,
        span: Span,
        declare: bool,
    ) -> Result<(), CodegenError> {
        match pattern {
            Pattern::Identifier(identifier) => {
                let slot = self.name_slot(&identifier.name)?;
                if declare {
                    self.builder.emit_init_name(value_reg, slot);
                    self.promote_fast_name(&identifier.name, value_reg);
                } else {
                    self.builder.emit_store_name(value_reg, slot);
                    self.sync_fast_name(&identifier.name, value_reg);
                }
                Ok(())
            }
            Pattern::Assignment(pattern) => match pattern.left.as_ref() {
                Pattern::Identifier(identifier) => {
                    let slot = self.name_slot(&identifier.name)?;
                    if declare {
                        self.builder.emit_init_name(value_reg, slot);
                        self.promote_fast_name(&identifier.name, value_reg);
                    } else {
                        self.builder.emit_store_name(value_reg, slot);
                        self.sync_fast_name(&identifier.name, value_reg);
                    }
                    Ok(())
                }
                other => Err(CodegenError::Unsupported {
                    feature: "complex for-of binding",
                    span: other.span(),
                }),
            },
            _ => Err(CodegenError::Unsupported {
                feature: "complex for-of binding",
                span,
            }),
        }
    }

    fn bind_assignment_target(
        &mut self,
        expression: &Expression,
        value_reg: u8,
    ) -> Result<(), CodegenError> {
        match expression {
            Expression::Identifier(identifier) => {
                let slot = self.name_slot(&identifier.name)?;
                self.builder.emit_store_name(value_reg, slot);
                self.sync_fast_name(&identifier.name, value_reg);
                Ok(())
            }
            Expression::Member(member) => {
                let (object_reg, key_reg, immediate_key) = self.compile_member_target(member)?;
                if let Some(key) = immediate_key {
                    self.builder.emit_set_prop(value_reg, object_reg, key);
                } else {
                    let key_reg = key_reg.expect("computed member key");
                    self.builder.emit_mov(ACC, value_reg);
                    self.builder.emit_set_prop_acc(object_reg, key_reg);
                }
                Ok(())
            }
            other => Err(CodegenError::Unsupported {
                feature: "for-of assignment target",
                span: other.span(),
            }),
        }
    }

    fn compile_variable_declaration(
        &mut self,
        declaration: &VariableDeclaration,
    ) -> Result<(), CodegenError> {
        for declarator in &declaration.declarations {
            self.compile_variable_declarator(declarator)?;
        }
        Ok(())
    }

    fn compile_variable_declarator(
        &mut self,
        declarator: &VariableDeclarator,
    ) -> Result<(), CodegenError> {
        match &declarator.pattern {
            Pattern::Identifier(identifier) => {
                let value_reg = if let Some(init) = &declarator.init {
                    self.compile_expression(init)?
                } else {
                    self.load_undefined(None)?
                };
                let slot = self.name_slot(&identifier.name)?;
                self.builder.emit_init_name(value_reg, slot);
                self.promote_fast_name(&identifier.name, value_reg);
                self.temp_top = value_reg.saturating_sub(1);
                Ok(())
            }
            Pattern::Assignment(pattern) => match pattern.left.as_ref() {
                Pattern::Identifier(identifier) => {
                    let value_reg = if let Some(init) = &declarator.init {
                        self.compile_expression(init)?
                    } else {
                        self.compile_expression(&pattern.right)?
                    };
                    let slot = self.name_slot(&identifier.name)?;
                    self.builder.emit_init_name(value_reg, slot);
                    self.promote_fast_name(&identifier.name, value_reg);
                    self.temp_top = value_reg.saturating_sub(1);
                    Ok(())
                }
                other => Err(CodegenError::Unsupported {
                    feature: "complex variable pattern",
                    span: other.span(),
                }),
            },
            other => Err(CodegenError::Unsupported {
                feature: "destructuring declarations",
                span: other.span(),
            }),
        }
    }

    fn compile_function_declaration(&mut self, function: &Function) -> Result<(), CodegenError> {
        let Some(identifier) = &function.id else {
            return Err(CodegenError::Unsupported {
                feature: "anonymous function declaration",
                span: function.span,
            });
        };
        let dst = self.alloc_temp(Some(function.span))?;
        let const_index = self.reserve_function_constant();
        self.pending_functions.push_back(PendingFunction {
            const_index,
            body: PendingFunctionBody::Function(function.clone()),
        });
        self.builder.emit_new_func(dst, const_index);
        let slot = self.name_slot(&identifier.name)?;
        self.builder.emit_init_name(dst, slot);
        self.promote_fast_name(&identifier.name, dst);
        self.temp_top = dst.saturating_sub(1);
        Ok(())
    }

    fn compile_function_params(&mut self, params: &[Pattern]) -> Result<(), CodegenError> {
        for (index, param) in params.iter().enumerate() {
            match param {
                Pattern::Identifier(identifier) => {
                    let reg = self.alloc_temp(Some(identifier.span))?;
                    self.builder.emit_load_arg(reg, index as u8);
                    let slot = self.name_slot(&identifier.name)?;
                    self.builder.emit_init_name(reg, slot);
                    self.promote_fast_name(&identifier.name, reg);
                    self.temp_top = reg.saturating_sub(1);
                }
                Pattern::Assignment(pattern) => match pattern.left.as_ref() {
                    Pattern::Identifier(identifier) => {
                        let reg = self.alloc_temp(Some(pattern.span))?;
                        self.builder.emit_load_arg(reg, index as u8);
                        let flag = self.alloc_temp(Some(pattern.span))?;
                        self.builder.emit_is_undef(flag, reg);
                        let skip_default = self.emit_placeholder_jmp_false(flag);
                        let default_reg = self.compile_expression(&pattern.right)?;
                        self.builder.emit_mov(reg, default_reg);
                        let after_default = self.builder.len();
                        self.patch_jump(
                            skip_default,
                            after_default,
                            JumpPatchKind::JmpFalse { reg: flag },
                        );
                        let slot = self.name_slot(&identifier.name)?;
                        self.builder.emit_init_name(reg, slot);
                        self.promote_fast_name(&identifier.name, reg);
                        self.temp_top = reg.saturating_sub(1);
                    }
                    other => {
                        return Err(CodegenError::Unsupported {
                            feature: "complex default parameter",
                            span: other.span(),
                        });
                    }
                },
                other => {
                    return Err(CodegenError::Unsupported {
                        feature: "complex function parameter",
                        span: other.span(),
                    });
                }
            }
        }
        Ok(())
    }

    fn compile_expression(&mut self, expression: &Expression) -> Result<u8, CodegenError> {
        match expression {
            Expression::Identifier(identifier) => self.compile_identifier_value(identifier),
            Expression::Literal(literal) => self.compile_literal(literal),
            Expression::This(_) => self.compile_this(expression.span()),
            Expression::Array(array) => self.compile_array_expression(array),
            Expression::Object(object) => self.compile_object_expression(object),
            Expression::Function(function) => self.compile_function_expression(function),
            Expression::ArrowFunction(function) => self.compile_arrow_function_expression(function),
            Expression::Unary(unary) => self.compile_unary_expression(unary),
            Expression::Update(update) => self.compile_update_expression(update),
            Expression::Binary(binary) => self.compile_binary_expression(binary),
            Expression::Logical(logical) => self.compile_logical_expression(logical),
            Expression::Assignment(assignment) => self.compile_assignment_expression(assignment),
            Expression::Conditional(conditional) => {
                self.compile_conditional_expression(conditional)
            }
            Expression::Sequence(sequence) => self.compile_sequence_expression(sequence),
            Expression::Call(call) => self.compile_call_expression(call),
            Expression::Member(member) => self.compile_member_expression(member),
            Expression::New(new) => self.compile_new_expression(new),
            Expression::Super(span) => Err(CodegenError::Unsupported {
                feature: "super",
                span: *span,
            }),
            Expression::PrivateIdentifier(identifier) => Err(CodegenError::Unsupported {
                feature: "private identifiers",
                span: identifier.span,
            }),
            Expression::Class(class) => Err(CodegenError::Unsupported {
                feature: "class expressions",
                span: class.span,
            }),
            Expression::TaggedTemplate(node) => Err(CodegenError::Unsupported {
                feature: "tagged templates",
                span: node.span,
            }),
            Expression::MetaProperty(node) => Err(CodegenError::Unsupported {
                feature: "meta properties",
                span: node.span,
            }),
            Expression::Yield(node) => Err(CodegenError::Unsupported {
                feature: "yield",
                span: node.span,
            }),
            Expression::Await(node) => Err(CodegenError::Unsupported {
                feature: "await",
                span: node.span,
            }),
        }
    }

    fn compile_identifier_value(&mut self, identifier: &Identifier) -> Result<u8, CodegenError> {
        if let Some(home_reg) = self.fast_name_reg(&identifier.name) {
            let reg = self.alloc_temp(Some(identifier.span))?;
            self.builder.emit_mov(reg, home_reg);
            return Ok(reg);
        }
        self.compile_identifier_current(identifier)
    }

    fn compile_identifier_current(&mut self, identifier: &Identifier) -> Result<u8, CodegenError> {
        if let Some(reg) = self.fast_name_reg(&identifier.name) {
            self.temp_top = self.temp_top.max(reg);
            return Ok(reg);
        }
        let reg = self.alloc_temp(Some(identifier.span))?;
        let slot = self.name_slot(&identifier.name)?;
        self.builder.emit_load_name(reg, slot);
        Ok(reg)
    }

    fn compile_readonly_expression(&mut self, expression: &Expression) -> Result<u8, CodegenError> {
        match expression {
            Expression::Identifier(identifier) => self.compile_identifier_current(identifier),
            _ => self.compile_expression(expression),
        }
    }

    fn compile_this(&mut self, span: Span) -> Result<u8, CodegenError> {
        let reg = self.alloc_temp(Some(span))?;
        self.builder.emit_load_this();
        self.builder.emit_mov(reg, ACC);
        Ok(reg)
    }

    fn compile_literal(&mut self, literal: &Literal) -> Result<u8, CodegenError> {
        match literal {
            Literal::Null(span) => {
                let reg = self.alloc_temp(Some(*span))?;
                self.builder.emit_load_null();
                self.builder.emit_mov(reg, ACC);
                Ok(reg)
            }
            Literal::Boolean(node) => {
                let reg = self.alloc_temp(Some(node.span))?;
                if node.value {
                    self.builder.emit_load_true(reg);
                } else {
                    self.builder.emit_load_false(reg);
                }
                Ok(reg)
            }
            Literal::Number(node) => {
                let reg = self.alloc_temp(Some(node.span))?;
                let value = parse_number_literal(node)?;
                if value.fract() == 0.0 && value >= i16::MIN as f64 && value <= i16::MAX as f64 {
                    self.builder.emit_load_i(reg, value as i16);
                } else {
                    let index = self.builder.add_constant(make_number(value));
                    self.builder.emit_load_k(reg, index);
                }
                Ok(reg)
            }
            Literal::String(node) => self.load_runtime_string(&node.value, node.span),
            Literal::Template(node) => self.compile_template_literal(node),
            Literal::RegExp(node) => Err(CodegenError::Unsupported {
                feature: "regexp literals",
                span: node.span,
            }),
        }
    }

    fn compile_template_literal(
        &mut self,
        literal: &gc3::TemplateLiteral,
    ) -> Result<u8, CodegenError> {
        let parts = parse_template_literal_parts(literal)?;
        let result = self.load_runtime_string("", literal.span)?;

        for part in parts {
            let value = match part {
                TemplatePart::Text(text) => {
                    if text.is_empty() {
                        continue;
                    }
                    self.load_runtime_string(&text, literal.span)?
                }
                TemplatePart::Expression(expression) => self.compile_expression(&expression)?,
            };
            self.builder.emit_add(result, value);
            self.builder.emit_mov(result, ACC);
            self.temp_top = result;
        }

        Ok(result)
    }

    fn compile_array_expression(
        &mut self,
        expression: &gc3::ArrayExpression,
    ) -> Result<u8, CodegenError> {
        let array_reg = self.alloc_temp(Some(expression.span))?;
        self.builder.emit_new_arr(
            array_reg,
            expression.elements.len().min(u8::MAX as usize) as u8,
        );

        for element in &expression.elements {
            match element {
                Some(gc3::ArrayElement::Expression(expression)) => {
                    let value = self.compile_expression(expression)?;
                    self.builder.emit_mov(ACC, value);
                    self.builder.emit_array_push_acc(array_reg);
                    self.temp_top = array_reg;
                }
                Some(gc3::ArrayElement::Spread { argument, .. }) => {
                    let source = self.compile_expression(argument)?;
                    self.builder.emit_spread(array_reg, source);
                    self.temp_top = array_reg;
                }
                None => {
                    let value = self.load_undefined(Some(expression.span))?;
                    self.builder.emit_mov(ACC, value);
                    self.builder.emit_array_push_acc(array_reg);
                    self.temp_top = array_reg;
                }
            }
        }

        Ok(array_reg)
    }

    fn compile_object_expression(
        &mut self,
        expression: &ObjectExpression,
    ) -> Result<u8, CodegenError> {
        let object_reg = self.alloc_temp(Some(expression.span))?;
        self.builder.emit_new_obj(object_reg);

        for property in &expression.properties {
            match property {
                ObjectProperty::Spread { span, .. } => {
                    return Err(CodegenError::Unsupported {
                        feature: "object spread",
                        span: *span,
                    });
                }
                ObjectProperty::Property {
                    key,
                    value,
                    kind,
                    span,
                    ..
                } => {
                    if !matches!(kind, ObjectPropertyKind::Init | ObjectPropertyKind::Method) {
                        return Err(CodegenError::Unsupported {
                            feature: "getters/setters in object literals",
                            span: *span,
                        });
                    }
                    let value_reg = self.compile_expression(value)?;
                    self.emit_store_property(object_reg, key, value_reg)?;
                    self.temp_top = object_reg;
                }
            }
        }

        Ok(object_reg)
    }

    fn compile_function_expression(&mut self, function: &Function) -> Result<u8, CodegenError> {
        let reg = self.alloc_temp(Some(function.span))?;
        let const_index = self.reserve_function_constant();
        self.pending_functions.push_back(PendingFunction {
            const_index,
            body: PendingFunctionBody::Function(function.clone()),
        });
        self.builder.emit_new_func(reg, const_index);
        Ok(reg)
    }

    fn compile_arrow_function_expression(
        &mut self,
        function: &ArrowFunction,
    ) -> Result<u8, CodegenError> {
        let reg = self.alloc_temp(Some(function.span))?;
        let const_index = self.reserve_function_constant();
        self.pending_functions.push_back(PendingFunction {
            const_index,
            body: PendingFunctionBody::Arrow(function.clone()),
        });
        self.builder.emit_new_func(reg, const_index);
        Ok(reg)
    }

    fn compile_unary_expression(
        &mut self,
        expression: &UnaryExpression,
    ) -> Result<u8, CodegenError> {
        match expression.operator {
            UnaryOperator::Typeof => match &expression.argument {
                Expression::Identifier(identifier) => {
                    let reg = self.alloc_temp(Some(expression.span))?;
                    let slot = self.name_slot(&identifier.name)?;
                    self.builder.emit_typeof_name(reg, slot);
                    Ok(reg)
                }
                argument => {
                    let reg = self.compile_expression(argument)?;
                    self.builder.emit_typeof(reg, reg);
                    Ok(reg)
                }
            },
            UnaryOperator::Positive => {
                let reg = self.compile_expression(&expression.argument)?;
                self.builder.emit_to_num(reg, reg);
                Ok(reg)
            }
            UnaryOperator::Negative => {
                let reg = self.compile_expression(&expression.argument)?;
                self.builder.emit_neg(reg);
                self.builder.emit_mov(reg, ACC);
                Ok(reg)
            }
            UnaryOperator::BitNot => {
                let reg = self.compile_expression(&expression.argument)?;
                self.builder.emit_bit_not(reg);
                self.builder.emit_mov(reg, ACC);
                Ok(reg)
            }
            UnaryOperator::LogicalNot => {
                let reg = self.compile_expression(&expression.argument)?;
                let false_branch = self.emit_placeholder_jmp_false(reg);
                self.builder.emit_load_false(reg);
                let end_jump = self.emit_placeholder_jmp();
                let true_start = self.builder.len();
                self.patch_jump(false_branch, true_start, JumpPatchKind::JmpFalse { reg });
                self.builder.emit_load_true(reg);
                let end = self.builder.len();
                self.patch_jump(end_jump, end, JumpPatchKind::Jmp);
                Ok(reg)
            }
            UnaryOperator::Void => {
                let reg = self.compile_expression(&expression.argument)?;
                let target = reg;
                let undef = self.load_undefined(Some(expression.span))?;
                if undef != target {
                    self.builder.emit_mov(target, undef);
                    self.temp_top = target;
                }
                Ok(target)
            }
            UnaryOperator::Delete => self.compile_delete_expression(expression),
        }
    }

    fn compile_delete_expression(
        &mut self,
        expression: &UnaryExpression,
    ) -> Result<u8, CodegenError> {
        match &expression.argument {
            Expression::Identifier(identifier) => {
                let reg = self.alloc_temp(Some(expression.span))?;
                if self.name_slots.contains_key(&identifier.name) {
                    self.builder.emit_load_false(reg);
                } else {
                    self.builder.emit_load_true(reg);
                }
                Ok(reg)
            }
            Expression::Member(member) => match &member.property {
                MemberProperty::Identifier(identifier) => {
                    let object = self.compile_expression(&member.object)?;
                    let reg = self.alloc_temp(Some(expression.span))?;
                    let slot = self.property_slot(&identifier.name)?;
                    self.builder.emit_delete_prop(reg, object, slot);
                    Ok(reg)
                }
                MemberProperty::Computed { .. } => Err(CodegenError::Unsupported {
                    feature: "computed delete",
                    span: expression.span,
                }),
                MemberProperty::PrivateName(identifier) => Err(CodegenError::Unsupported {
                    feature: "private members",
                    span: identifier.span,
                }),
            },
            _ => {
                let reg = self.compile_expression(&expression.argument)?;
                self.builder.emit_load_true(reg);
                Ok(reg)
            }
        }
    }

    fn compile_update_expression(
        &mut self,
        expression: &UpdateExpression,
    ) -> Result<u8, CodegenError> {
        match &expression.argument {
            Expression::Identifier(identifier) => {
                let current = self.compile_identifier_current(identifier)?;
                match expression.operator {
                    UpdateOperator::Increment => self.builder.emit_inc(current),
                    UpdateOperator::Decrement => self.builder.emit_dec(current),
                }

                if expression.prefix {
                    self.builder.emit_mov(current, ACC);
                    let slot = self.name_slot(&identifier.name)?;
                    self.builder.emit_store_name(current, slot);
                    self.sync_fast_name(&identifier.name, current);
                    Ok(current)
                } else {
                    let updated = self.alloc_temp(Some(expression.span))?;
                    self.builder.emit_mov(updated, ACC);
                    let slot = self.name_slot(&identifier.name)?;
                    self.builder.emit_store_name(updated, slot);
                    self.sync_fast_name(&identifier.name, updated);
                    self.temp_top = current;
                    Ok(current)
                }
            }
            Expression::Member(member) => {
                let (object_reg, key_reg, immediate_key) = self.compile_member_target(member)?;
                let current = if let Some(key) = immediate_key {
                    self.builder.emit_get_prop(object_reg, object_reg, key);
                    object_reg
                } else {
                    let key_reg = key_reg.expect("computed member key");
                    self.builder.emit_get_prop_acc(object_reg, key_reg);
                    self.builder.emit_mov(object_reg, ACC);
                    object_reg
                };

                match expression.operator {
                    UpdateOperator::Increment => self.builder.emit_inc(current),
                    UpdateOperator::Decrement => self.builder.emit_dec(current),
                }

                if expression.prefix {
                    self.builder.emit_mov(current, ACC);
                    if let Some(key) = immediate_key {
                        self.builder.emit_set_prop(current, object_reg, key);
                    } else {
                        let key_reg = key_reg.expect("computed member key");
                        self.builder.emit_mov(ACC, current);
                        self.builder.emit_set_prop_acc(object_reg, key_reg);
                    }
                    Ok(current)
                } else {
                    let updated = self.alloc_temp(Some(expression.span))?;
                    self.builder.emit_mov(updated, ACC);
                    if let Some(key) = immediate_key {
                        self.builder.emit_set_prop(updated, object_reg, key);
                    } else {
                        let key_reg = key_reg.expect("computed member key");
                        self.builder.emit_mov(ACC, updated);
                        self.builder.emit_set_prop_acc(object_reg, key_reg);
                    }
                    self.temp_top = current;
                    Ok(current)
                }
            }
            other => Err(CodegenError::Unsupported {
                feature: "update target",
                span: other.span(),
            }),
        }
    }

    fn compile_binary_expression(
        &mut self,
        expression: &BinaryExpression,
    ) -> Result<u8, CodegenError> {
        let left = self.compile_expression(&expression.left)?;
        let right = self.compile_expression(&expression.right)?;

        match expression.operator {
            BinaryOperator::Add => {
                self.builder.emit_add(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::Subtract => {
                self.builder.emit_mov(ACC, left);
                self.builder.emit_sub_acc(right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::Multiply => {
                self.builder.emit_mov(ACC, left);
                self.builder.emit_mul_acc(right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::Divide => {
                self.builder.emit_mov(ACC, left);
                self.builder.emit_div_acc(right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::Modulo => {
                self.builder.emit_mod(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::Exponentiate => {
                self.builder.emit_pow(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::LeftShift => {
                self.builder.emit_shl(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::SignedRightShift => {
                self.builder.emit_shr(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::UnsignedRightShift => {
                self.builder.emit_ushr(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::LessThan => {
                self.builder.emit_lt(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::LessThanOrEqual => {
                self.builder.emit_lte(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::GreaterThan => {
                self.builder.emit_lt(right, left);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::GreaterThanOrEqual => {
                self.builder.emit_lte(right, left);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::Equality => {
                self.builder.emit_eq(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::StrictEquality => {
                self.builder.emit_strict_eq(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::Inequality => {
                self.builder.emit_eq(left, right);
                self.builder.emit_mov(left, ACC);
                let false_branch = self.emit_placeholder_jmp_false(left);
                self.builder.emit_load_false(left);
                let end_jump = self.emit_placeholder_jmp();
                let true_start = self.builder.len();
                self.patch_jump(
                    false_branch,
                    true_start,
                    JumpPatchKind::JmpFalse { reg: left },
                );
                self.builder.emit_load_true(left);
                let end = self.builder.len();
                self.patch_jump(end_jump, end, JumpPatchKind::Jmp);
            }
            BinaryOperator::StrictInequality => {
                self.builder.emit_strict_neq(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::BitwiseAnd => {
                self.builder.emit_bit_and(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::BitwiseOr => {
                self.builder.emit_bit_or(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::BitwiseXor => {
                self.builder.emit_bit_xor(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::In => {
                self.builder.emit_in(left, right);
                self.builder.emit_mov(left, ACC);
            }
            BinaryOperator::Instanceof => {
                self.builder.emit_instanceof(left, right);
                self.builder.emit_mov(left, ACC);
            }
        }

        self.temp_top = left;
        Ok(left)
    }

    fn compile_logical_expression(
        &mut self,
        expression: &LogicalExpression,
    ) -> Result<u8, CodegenError> {
        let left = self.compile_expression(&expression.left)?;
        let right = self.compile_expression(&expression.right)?;

        match expression.operator {
            LogicalOperator::And => self.builder.emit_logical_and(left, right),
            LogicalOperator::Or => self.builder.emit_logical_or(left, right),
            LogicalOperator::NullishCoalescing => self.builder.emit_nullish_coalesce(left, right),
        }
        self.builder.emit_mov(left, ACC);
        self.temp_top = left;
        Ok(left)
    }

    fn compile_assignment_expression(
        &mut self,
        expression: &AssignmentExpression,
    ) -> Result<u8, CodegenError> {
        match &expression.left {
            Expression::Identifier(identifier) => {
                let value = match expression.operator {
                    AssignmentOperator::Assign => self.compile_expression(&expression.right)?,
                    AssignmentOperator::AddAssign => {
                        let left = self.compile_identifier_current(identifier)?;
                        let right = self.compile_expression(&expression.right)?;
                        self.builder.emit_add(left, right);
                        self.builder.emit_mov(left, ACC);
                        self.temp_top = left;
                        left
                    }
                    AssignmentOperator::SubAssign => {
                        let left = self.compile_identifier_current(identifier)?;
                        let right = self.compile_expression(&expression.right)?;
                        self.builder.emit_mov(ACC, left);
                        self.builder.emit_sub_acc(right);
                        self.builder.emit_mov(left, ACC);
                        self.temp_top = left;
                        left
                    }
                    AssignmentOperator::MulAssign => {
                        let left = self.compile_identifier_current(identifier)?;
                        let right = self.compile_expression(&expression.right)?;
                        self.builder.emit_mov(ACC, left);
                        self.builder.emit_mul_acc(right);
                        self.builder.emit_mov(left, ACC);
                        self.temp_top = left;
                        left
                    }
                    AssignmentOperator::DivAssign => {
                        let left = self.compile_identifier_current(identifier)?;
                        let right = self.compile_expression(&expression.right)?;
                        self.builder.emit_mov(ACC, left);
                        self.builder.emit_div_acc(right);
                        self.builder.emit_mov(left, ACC);
                        self.temp_top = left;
                        left
                    }
                    _ => {
                        return Err(CodegenError::Unsupported {
                            feature: "assignment operator",
                            span: expression.span,
                        });
                    }
                };

                let slot = self.name_slot(&identifier.name)?;
                self.builder.emit_store_name(value, slot);
                self.sync_fast_name(&identifier.name, value);
                self.temp_top = value;
                Ok(value)
            }
            Expression::Member(member) => {
                let (object_reg, key_reg, immediate_key) = self.compile_member_target(member)?;
                let value = match expression.operator {
                    AssignmentOperator::Assign => self.compile_expression(&expression.right)?,
                    AssignmentOperator::AddAssign
                    | AssignmentOperator::SubAssign
                    | AssignmentOperator::MulAssign
                    | AssignmentOperator::DivAssign => {
                        let current = if let Some(key) = immediate_key {
                            self.builder.emit_get_prop(object_reg, object_reg, key);
                            object_reg
                        } else {
                            let key_reg = key_reg.expect("computed member key");
                            self.builder.emit_get_prop_acc(object_reg, key_reg);
                            self.builder.emit_mov(object_reg, ACC);
                            object_reg
                        };
                        let rhs = self.compile_expression(&expression.right)?;
                        match expression.operator {
                            AssignmentOperator::AddAssign => self.builder.emit_add(current, rhs),
                            AssignmentOperator::SubAssign => {
                                self.builder.emit_mov(ACC, current);
                                self.builder.emit_sub_acc(rhs);
                            }
                            AssignmentOperator::MulAssign => {
                                self.builder.emit_mov(ACC, current);
                                self.builder.emit_mul_acc(rhs);
                            }
                            AssignmentOperator::DivAssign => {
                                self.builder.emit_mov(ACC, current);
                                self.builder.emit_div_acc(rhs);
                            }
                            _ => unreachable!(),
                        }
                        self.builder.emit_mov(current, ACC);
                        self.temp_top = current;
                        current
                    }
                    _ => {
                        return Err(CodegenError::Unsupported {
                            feature: "assignment operator",
                            span: expression.span,
                        });
                    }
                };

                if let Some(key) = immediate_key {
                    self.builder.emit_set_prop(value, object_reg, key);
                } else {
                    let key_reg = key_reg.expect("computed member key");
                    self.builder.emit_mov(ACC, value);
                    self.builder.emit_set_prop_acc(object_reg, key_reg);
                }
                self.temp_top = value;
                Ok(value)
            }
            other => Err(CodegenError::Unsupported {
                feature: "assignment target",
                span: other.span(),
            }),
        }
    }

    fn compile_conditional_expression(
        &mut self,
        expression: &ConditionalExpression,
    ) -> Result<u8, CodegenError> {
        let test = self.compile_expression(&expression.test)?;
        let target = test;
        let false_jump = self.emit_placeholder_jmp_false(test);

        let consequent = self.compile_expression(&expression.consequent)?;
        if consequent != target {
            self.builder.emit_mov(target, consequent);
        }
        let end_jump = self.emit_placeholder_jmp();

        let alternate_start = self.builder.len();
        self.patch_jump(
            false_jump,
            alternate_start,
            JumpPatchKind::JmpFalse { reg: test },
        );

        let alternate = self.compile_expression(&expression.alternate)?;
        if alternate != target {
            self.builder.emit_mov(target, alternate);
        }

        let end = self.builder.len();
        self.patch_jump(end_jump, end, JumpPatchKind::Jmp);
        self.temp_top = target;
        Ok(target)
    }

    fn compile_sequence_expression(
        &mut self,
        expression: &SequenceExpression,
    ) -> Result<u8, CodegenError> {
        let mut last = self.load_undefined(Some(expression.span))?;
        for expr in &expression.expressions {
            last = self.compile_expression(expr)?;
        }
        Ok(last)
    }

    fn compile_fixed_call_arguments(
        &mut self,
        callee: u8,
        arguments: &[CallArgument],
        span: Span,
    ) -> Result<u8, CodegenError> {
        for (index, argument) in arguments.iter().enumerate() {
            match argument {
                CallArgument::Expression(expression) => {
                    let arg_reg = self.compile_expression(expression)?;
                    let expected = callee as usize + 1 + index;
                    if expected >= ACC as usize {
                        return Err(CodegenError::RegisterOverflow { span: Some(span) });
                    }
                    if arg_reg != expected as u8 {
                        self.builder.emit_mov(expected as u8, arg_reg);
                        self.temp_top = self.temp_top.max(expected as u8);
                    }
                }
                CallArgument::Spread { span, .. } => {
                    return Err(CodegenError::Unsupported {
                        feature: "mixed spread calls",
                        span: *span,
                    });
                }
            }
        }

        Ok(arguments.len().min(u8::MAX as usize) as u8)
    }

    fn compile_call_expression(&mut self, expression: &CallExpression) -> Result<u8, CodegenError> {
        if expression.optional {
            return Err(CodegenError::Unsupported {
                feature: "optional call",
                span: expression.span,
            });
        }

        if let Expression::Member(member) = &expression.callee {
            if expression.arguments.is_empty() {
                let object = self.compile_expression(&member.object)?;
                match &member.property {
                    MemberProperty::Identifier(identifier) => {
                        let slot = self.property_slot(&identifier.name)?;
                        self.builder.emit_call_method_ic(object, slot);
                        self.builder.emit_mov(object, ACC);
                        self.temp_top = object;
                        return Ok(object);
                    }
                    MemberProperty::Computed { expression, .. } => {
                        let key = self.compile_expression(expression)?;
                        self.builder.emit_get_prop_acc_call(object, key);
                        self.builder.emit_mov(object, ACC);
                        self.temp_top = object;
                        return Ok(object);
                    }
                    MemberProperty::PrivateName(identifier) => {
                        return Err(CodegenError::Unsupported {
                            feature: "private method calls",
                            span: identifier.span,
                        });
                    }
                }
            }

            if expression.arguments.len() <= 2
                && expression
                    .arguments
                    .iter()
                    .all(|argument| matches!(argument, CallArgument::Expression(_)))
                && let MemberProperty::Identifier(identifier) = &member.property
            {
                let object = self.compile_expression(&member.object)?;
                let arg_count = self.compile_fixed_call_arguments(
                    object,
                    &expression.arguments,
                    expression.span,
                )?;
                let slot = self.property_slot(&identifier.name)?;
                match arg_count {
                    1 => self.builder.emit_call_method1(object, u16::from(slot)),
                    2 => self.builder.emit_call_method2(object, u16::from(slot)),
                    _ => unreachable!("member fast path only handles one or two arguments"),
                }
                self.builder.emit_mov(object, ACC);
                self.temp_top = object;
                return Ok(object);
            }
        }

        if !matches!(expression.callee, Expression::Member(_))
            && let Some((source, imm)) = extract_call1_sub_i_arg(&expression.arguments)?
        {
            let callee = self.compile_expression(&expression.callee)?;
            let source = self.compile_readonly_expression(source)?;
            self.builder.emit_call1_sub_i(callee, source, imm);
            self.builder.emit_mov(callee, ACC);
            self.temp_top = callee;
            return Ok(callee);
        }

        let callee = self.compile_expression(&expression.callee)?;

        if let [CallArgument::Spread { argument, .. }] = expression.arguments.as_slice() {
            let array_reg = self.compile_expression(argument)?;
            if array_reg != callee + 1 {
                return Err(CodegenError::Unsupported {
                    feature: "spread call with non-contiguous arguments",
                    span: expression.span,
                });
            }
            self.builder.emit_call_var(callee, array_reg);
            self.builder.emit_mov(callee, ACC);
            self.temp_top = callee;
            return Ok(callee);
        }

        let arg_count =
            self.compile_fixed_call_arguments(callee, &expression.arguments, expression.span)?;
        self.builder.emit_call(callee, arg_count);
        self.builder.emit_mov(callee, ACC);
        self.temp_top = callee;
        Ok(callee)
    }

    fn compile_member_expression(
        &mut self,
        expression: &MemberExpression,
    ) -> Result<u8, CodegenError> {
        if expression.optional {
            return Err(CodegenError::Unsupported {
                feature: "optional chaining",
                span: expression.span,
            });
        }

        let object = self.compile_expression(&expression.object)?;
        match &expression.property {
            MemberProperty::Identifier(identifier) => {
                let slot = self.property_slot(&identifier.name)?;
                self.builder.emit_get_prop(object, object, slot);
                Ok(object)
            }
            MemberProperty::Computed { expression, .. } => {
                let key = self.compile_expression(expression)?;
                self.builder.emit_get_prop_acc(object, key);
                self.builder.emit_mov(object, ACC);
                self.temp_top = object;
                Ok(object)
            }
            MemberProperty::PrivateName(identifier) => Err(CodegenError::Unsupported {
                feature: "private members",
                span: identifier.span,
            }),
        }
    }

    fn compile_new_expression(&mut self, expression: &NewExpression) -> Result<u8, CodegenError> {
        let callee = self.compile_expression(&expression.callee)?;

        if let [CallArgument::Spread { span, .. }] = expression.arguments.as_slice() {
            return Err(CodegenError::Unsupported {
                feature: "spread in `new` expressions",
                span: *span,
            });
        }

        for argument in &expression.arguments {
            match argument {
                CallArgument::Expression(expression) => {
                    let _ = self.compile_expression(expression)?;
                }
                CallArgument::Spread { span, .. } => {
                    return Err(CodegenError::Unsupported {
                        feature: "spread in `new` expressions",
                        span: *span,
                    });
                }
            }
        }

        self.builder.emit_construct(
            callee,
            expression.arguments.len().min(u8::MAX as usize) as u8,
        );
        self.builder.emit_mov(callee, ACC);
        self.temp_top = callee;
        Ok(callee)
    }

    fn compile_member_target(
        &mut self,
        member: &MemberExpression,
    ) -> Result<(u8, Option<u8>, Option<u8>), CodegenError> {
        if member.optional {
            return Err(CodegenError::Unsupported {
                feature: "optional chaining",
                span: member.span,
            });
        }

        let object_reg = self.compile_expression(&member.object)?;
        match &member.property {
            MemberProperty::Identifier(identifier) => {
                let slot = self.property_slot(&identifier.name)?;
                Ok((object_reg, None, Some(slot)))
            }
            MemberProperty::Computed { expression, .. } => {
                let key_reg = self.compile_expression(expression)?;
                Ok((object_reg, Some(key_reg), None))
            }
            MemberProperty::PrivateName(identifier) => Err(CodegenError::Unsupported {
                feature: "private members",
                span: identifier.span,
            }),
        }
    }

    fn emit_store_property(
        &mut self,
        object_reg: u8,
        key: &PropertyKey,
        value_reg: u8,
    ) -> Result<(), CodegenError> {
        match key {
            PropertyKey::Identifier(identifier) => {
                let slot = self.property_slot(&identifier.name)?;
                self.builder.emit_set_prop(value_reg, object_reg, slot);
            }
            PropertyKey::String(StringLiteral { value, .. }) => {
                let slot = self.property_slot(value)?;
                self.builder.emit_set_prop(value_reg, object_reg, slot);
            }
            PropertyKey::Number(number) => {
                let key_reg = self.compile_numeric_key(number)?;
                self.builder.emit_mov(ACC, value_reg);
                self.builder.emit_set_prop_acc(object_reg, key_reg);
            }
            PropertyKey::Computed { expression, .. } => {
                let key_reg = self.compile_expression(expression)?;
                self.builder.emit_mov(ACC, value_reg);
                self.builder.emit_set_prop_acc(object_reg, key_reg);
            }
            PropertyKey::PrivateName(identifier) => {
                return Err(CodegenError::Unsupported {
                    feature: "private object properties",
                    span: identifier.span,
                });
            }
        }
        Ok(())
    }

    fn compile_numeric_key(&mut self, literal: &NumberLiteral) -> Result<u8, CodegenError> {
        let reg = self.alloc_temp(Some(literal.span))?;
        let value = parse_number_literal(literal)?;
        if value.fract() == 0.0 && value >= i16::MIN as f64 && value <= i16::MAX as f64 {
            self.builder.emit_load_i(reg, value as i16);
        } else {
            let index = self.builder.add_constant(make_number(value));
            self.builder.emit_load_k(reg, index);
        }
        Ok(reg)
    }

    fn load_runtime_string(&mut self, value: &str, span: Span) -> Result<u8, CodegenError> {
        let reg = self.alloc_temp(Some(span))?;
        let index = self.builder.add_constant(make_undefined());
        self.string_constants.push((index, value.to_owned()));
        self.builder.emit_load_k(reg, index);
        Ok(reg)
    }

    fn load_undefined(&mut self, span: Option<Span>) -> Result<u8, CodegenError> {
        let reg = self.alloc_temp(span)?;
        let index = *self
            .undefined_const
            .get_or_insert_with(|| self.builder.add_constant(make_undefined()));
        self.builder.emit_load_k(reg, index);
        Ok(reg)
    }

    fn reserve_function_constant(&mut self) -> u16 {
        self.builder.add_constant(make_number(0.0))
    }

    fn alloc_temp(&mut self, span: Option<Span>) -> Result<u8, CodegenError> {
        loop {
            if self.temp_top >= MAX_TEMP_REG {
                return Err(CodegenError::RegisterOverflow { span });
            }
            self.temp_top += 1;
            if !self
                .fast_name_regs
                .values()
                .any(|&reg| reg == self.temp_top)
            {
                return Ok(self.temp_top);
            }
        }
    }

    fn emit_placeholder_jmp(&mut self) -> usize {
        let pos = self.builder.len();
        self.builder.emit_jmp(0);
        pos
    }

    fn emit_placeholder_jmp_false(&mut self, reg: u8) -> usize {
        let pos = self.builder.len();
        self.builder.emit_jmp_false(reg, 0);
        pos
    }

    fn emit_placeholder_jmp_lte_false(&mut self, lhs: u8, rhs: u8) -> usize {
        let pos = self.builder.len();
        self.builder.emit_jmp_lte_false(lhs, rhs, 0);
        pos
    }

    fn compile_condition_jump_false(
        &mut self,
        expression: &Expression,
    ) -> Result<(usize, JumpPatchKind, u8), CodegenError> {
        if let Expression::Binary(binary) = expression
            && binary.operator == BinaryOperator::LessThanOrEqual
        {
            let lhs = self.compile_readonly_expression(&binary.left)?;
            let rhs = self.compile_readonly_expression(&binary.right)?;
            let jump = self.emit_placeholder_jmp_lte_false(lhs, rhs);
            return Ok((jump, JumpPatchKind::JmpLteFalse { lhs, rhs }, lhs.max(rhs)));
        }

        let reg = self.compile_expression(expression)?;
        let jump = self.emit_placeholder_jmp_false(reg);
        Ok((jump, JumpPatchKind::JmpFalse { reg }, reg))
    }

    fn patch_jump(&mut self, pos: usize, target: usize, kind: JumpPatchKind) {
        self.jump_patches.push(JumpPatch { pos, target, kind });
    }

    fn patch_loop_breaks(&mut self, patches: Vec<usize>, target: usize) {
        for pos in patches {
            self.patch_jump(pos, target, JumpPatchKind::Jmp);
        }
    }

    fn patch_loop_continues(&mut self, patches: Vec<usize>, target: usize) {
        for pos in patches {
            self.patch_jump(pos, target, JumpPatchKind::Jmp);
        }
    }

    fn name_slot(&mut self, name: &str) -> Result<u16, CodegenError> {
        if let Some(&slot) = self.name_slots.get(name) {
            return Ok(slot);
        }

        let slot =
            u16::try_from(self.name_slots.len()).map_err(|_| CodegenError::NameOverflow {
                name: name.to_owned(),
            })?;
        self.name_slots.insert(name.to_owned(), slot);
        self.names.push(name.to_owned());
        Ok(slot)
    }

    fn property_slot(&mut self, name: &str) -> Result<u8, CodegenError> {
        if let Some(&slot) = self.property_slots.get(name) {
            return Ok(slot);
        }

        let slot = u8::try_from(self.property_slots.len()).map_err(|_| {
            CodegenError::PropertyOverflow {
                name: name.to_owned(),
            }
        })?;
        self.property_slots.insert(name.to_owned(), slot);
        self.properties.push(name.to_owned());
        Ok(slot)
    }

    fn fast_name_reg(&self, name: &str) -> Option<u8> {
        if !self.fast_name_bindings_enabled {
            return None;
        }
        self.fast_name_regs.get(name).copied()
    }

    fn enter_fast_name_scope(&mut self) {
        if self.fast_name_bindings_enabled {
            self.fast_name_scope_stack.push(Vec::new());
        }
    }

    fn leave_fast_name_scope(&mut self) {
        if !self.fast_name_bindings_enabled {
            return;
        }

        let Some(mut bindings) = self.fast_name_scope_stack.pop() else {
            return;
        };

        while let Some((name, previous)) = bindings.pop() {
            if let Some(reg) = previous {
                self.fast_name_regs.insert(name, reg);
            } else {
                self.fast_name_regs.remove(&name);
            }
        }
    }

    fn record_fast_name_scope_change(&mut self, name: &str) {
        if !self.fast_name_bindings_enabled {
            return;
        }

        let Some(scope) = self.fast_name_scope_stack.last_mut() else {
            return;
        };
        if scope.iter().any(|(existing, _)| existing == name) {
            return;
        }

        scope.push((name.to_owned(), self.fast_name_regs.get(name).copied()));
    }

    fn promote_fast_name(&mut self, name: &str, value_reg: u8) {
        if !self.fast_name_bindings_enabled {
            return;
        }

        self.record_fast_name_scope_change(name);
        self.fast_name_regs.insert(name.to_owned(), value_reg);
    }

    fn sync_fast_name(&mut self, name: &str, value_reg: u8) {
        let Some(home_reg) = self.fast_name_reg(name) else {
            return;
        };
        if home_reg != value_reg {
            self.builder.emit_mov(home_reg, value_reg);
            self.temp_top = self.temp_top.max(home_reg);
        }
    }

}

fn parse_number_literal(literal: &NumberLiteral) -> Result<f64, CodegenError> {
    let raw = literal.raw.replace('_', "");
    let value = if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok().map(|value| value as f64)
    } else if let Some(bin) = raw.strip_prefix("0b").or_else(|| raw.strip_prefix("0B")) {
        i64::from_str_radix(bin, 2).ok().map(|value| value as f64)
    } else if let Some(oct) = raw.strip_prefix("0o").or_else(|| raw.strip_prefix("0O")) {
        i64::from_str_radix(oct, 8).ok().map(|value| value as f64)
    } else {
        raw.parse::<f64>().ok()
    };

    value.ok_or_else(|| CodegenError::NumericLiteral {
        raw: literal.raw.clone(),
        span: literal.span,
    })
}

fn extract_call1_sub_i_arg(
    arguments: &[CallArgument],
) -> Result<Option<(&Expression, i8)>, CodegenError> {
    let [CallArgument::Expression(Expression::Binary(binary))] = arguments else {
        return Ok(None);
    };
    if binary.operator != BinaryOperator::Subtract {
        return Ok(None);
    }
    let Expression::Literal(Literal::Number(number)) = &binary.right else {
        return Ok(None);
    };
    let value = parse_number_literal(number)?;
    if value.fract() != 0.0 || value < i8::MIN as f64 || value > i8::MAX as f64 {
        return Ok(None);
    }

    Ok(Some((&binary.left, value as i8)))
}

#[derive(Debug)]
enum TemplatePart {
    Text(String),
    Expression(Expression),
}

fn parse_template_literal_parts(
    literal: &gc3::TemplateLiteral,
) -> Result<Vec<TemplatePart>, CodegenError> {
    let value = literal.value.as_str();
    let mut parts = Vec::new();
    let mut text_start = 0usize;
    let mut search_start = 0usize;

    while let Some(relative_start) = value[search_start..].find("${") {
        let expr_start = search_start + relative_start;
        if expr_start > text_start {
            parts.push(TemplatePart::Text(value[text_start..expr_start].to_owned()));
        }

        let body_start = expr_start + 2;
        let mut matched = None;
        for (relative_end, ch) in value[body_start..].char_indices() {
            if ch != '}' {
                continue;
            }

            let expression_source = &value[body_start..body_start + relative_end];
            match parse_template_expression(expression_source, literal.span) {
                Ok(expression) => {
                    matched = Some((body_start + relative_end + ch.len_utf8(), expression));
                    break;
                }
                Err(CodegenError::Parse(_)) => continue,
                Err(error) => return Err(error),
            }
        }

        let Some((next_index, expression)) = matched else {
            return Err(CodegenError::Unsupported {
                feature: "template literal interpolations",
                span: literal.span,
            });
        };

        parts.push(TemplatePart::Expression(expression));
        text_start = next_index;
        search_start = next_index;
    }

    if text_start < value.len() {
        parts.push(TemplatePart::Text(value[text_start..].to_owned()));
    }

    Ok(parts)
}

fn parse_template_expression(source: &str, span: Span) -> Result<Expression, CodegenError> {
    let wrapped = format!("({source});");
    let program = gc3::parse(&wrapped)?;
    let mut statements = program.body.into_iter();
    match statements.next() {
        Some(Statement::Expression(ExpressionStatement { expression, .. }))
            if statements.next().is_none() =>
        {
            Ok(expression)
        }
        _ => Err(CodegenError::Unsupported {
            feature: "template literal interpolations",
            span,
        }),
    }
}

fn offset_to(target: usize, current_len: usize) -> Result<i16, CodegenError> {
    i16::try_from(target as isize - current_len as isize - 1).map_err(|_| {
        CodegenError::Unsupported {
            feature: "jump offset out of range",
            span: Span::default(),
        }
    })
}

fn encode_asbx(opcode: Opcode, a: u8, sbx: i16) -> u32 {
    (((sbx as u16) as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}
