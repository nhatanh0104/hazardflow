//! Verilog IR.

use itertools::Itertools;

use crate::compiler::prelude::Shape;
use crate::compiler::{BinaryOp, PortDecls, UnaryOp};
use crate::utils::{indent, join_options};

const INDENT: usize = 4;

/// Module.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Module {
    /// Module name.
    pub name: String,

    /// Port declarations.
    pub port_decls: Vec<PortDeclaration>,

    /// Module items.
    pub module_items: Vec<ModuleItem>,
}

impl ToString for Module {
    fn to_string(&self) -> String {
        format!(
            "module {}\n(\n{}\n);\n\ngenerate\n{}\nendgenerate\nendmodule",
            self.name,
            indent(
                self.port_decls.iter().map(|port_decl| port_decl.to_string()).collect::<Vec<_>>().join(",\n"),
                INDENT
            ),
            gen_verilog_module(&self.module_items)
        )
    }
}

/// Module item.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ModuleItem {
    /// Declarations.
    Declarations(Vec<Declaration>),

    /// Continuous assignments.
    ContinuousAssigns(Vec<ContinuousAssign>),

    /// Module instantiation.
    ModuleInstantiation(ModuleInstantiation),

    /// Always construct.
    AlwaysConstruct(String, Vec<Statement>),

    /// Comment. (Comment before modules, comment after modules, modules)
    Commented(String, Option<String>, Vec<ModuleItem>),
}

impl ModuleItem {
    /// Wrap module items into with comment
    pub fn comment(comment_before: String, comment_after: Option<String>, items: Vec<Self>) -> ModuleItem {
        Self::Commented(comment_before, comment_after, items)
    }
}

impl ToString for ModuleItem {
    fn to_string(&self) -> String {
        match self {
            ModuleItem::Declarations(decls) => decls.iter().map(|decl| decl.to_string()).collect::<Vec<_>>().join("\n"),
            ModuleItem::ContinuousAssigns(conts) => gen_verilog_conts(conts),
            ModuleItem::ModuleInstantiation(module_inst) => module_inst.to_string(),
            ModuleItem::AlwaysConstruct(event, stmts) => {
                format!(
                    "{} begin\n{}\nend",
                    event,
                    indent(stmts.iter().map(|stmt| stmt.to_string()).collect::<Vec<_>>().join("\n"), INDENT)
                )
            }
            ModuleItem::Commented(comment_before, comment_after, items) => {
                format!(
                    "/*\n{}\n*/\n{}{}",
                    indent(comment_before.clone(), INDENT),
                    items.iter().map(|item| item.to_string()).collect::<Vec<_>>().join("\n\n"),
                    comment_after.as_ref().map_or("".to_string(), |c| format!("\n/* {} */", c))
                )
            }
        }
    }
}

/// Generates Verilog code for module items.
pub fn gen_verilog_module(module: &[ModuleItem]) -> String {
    module.iter().map(|item| item.to_string()).collect::<Vec<_>>().join("\n\n")
}

/// Port declaration.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PortDeclaration {
    /// Input declaration.
    Input(usize, String),

    /// Output declaration.
    Output(usize, String),
}

impl ToString for PortDeclaration {
    fn to_string(&self) -> String {
        match self {
            Self::Input(width, ident) => {
                if *width > 1 {
                    format!("input wire [{}-1:0] {}", width, ident)
                } else {
                    format!("input wire {}", ident)
                }
            }
            Self::Output(width, ident) => {
                if *width > 1 {
                    format!("output wire [{}-1:0] {}", width, ident)
                } else {
                    format!("output wire {}", ident)
                }
            }
        }
    }
}

impl PortDeclaration {
    /// Creates new input port declaration.
    pub fn input(width: usize, ident: String) -> Self {
        Self::Input(width, ident)
    }

    /// Creates new output port declaration.
    pub fn output(width: usize, ident: String) -> Self {
        Self::Output(width, ident)
    }

    /// flip
    pub fn filp(self) -> Self {
        match self {
            PortDeclaration::Input(sz, name) => PortDeclaration::Output(sz, name),
            PortDeclaration::Output(sz, name) => PortDeclaration::Input(sz, name),
        }
    }

    /// name
    pub fn name(&self) -> String {
        match self {
            PortDeclaration::Input(_, name) | PortDeclaration::Output(_, name) => name.clone(),
        }
    }
}

/// Declaration.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Declaration {
    /// Net declaration.
    Net(Shape, String),

    /// Reg declaration.
    Reg(Shape, String, Option<Expression>),

    /// Integer declaration.
    Integer(String),
}

impl Declaration {
    /// Returns the shape of the declaration.
    pub fn shape(&self) -> Shape {
        match self {
            Declaration::Net(shape, _) => shape.clone(),
            Declaration::Reg(shape, ..) => shape.clone(),
            Declaration::Integer(_) => panic!(),
        }
    }

    /// Net declaration.
    #[inline]
    pub fn net(shape: Shape, ident: String) -> Self {
        Declaration::Net(shape, ident)
    }

    /// Reg declaration with no init value.
    #[inline]
    pub fn reg(shape: Shape, ident: String) -> Self {
        Declaration::Reg(shape, ident, None)
    }

    /// TODO: Documentation
    #[inline]
    pub fn with_init(self, init: Expression) -> Self {
        match self {
            Declaration::Reg(shape, ident, None) => {
                assert_eq!(shape.dim(), 1);
                Declaration::Reg(shape, ident, Some(init))
            }
            _ => panic!("with_init: self is not reg"),
        }
    }

    /// Reg declarations with value type.
    pub fn reg_with_typ(typ: PortDecls, prefix: Option<String>) -> Vec<Declaration> {
        typ.iter()
            .map(|(name, shape)| Declaration::reg(shape, join_options("_", [prefix.clone(), name]).unwrap()))
            .collect()
    }

    /// Returns the identifier of the declaration.
    pub fn ident(&self) -> Expression {
        Expression::ident(self.name())
    }

    /// name
    pub fn name(&self) -> String {
        match self {
            Declaration::Net(_, x) => x.clone(),
            Declaration::Reg(_, x, _) => x.clone(),
            Declaration::Integer(x) => x.clone(),
        }
    }

    /// Integer declaration.
    #[inline]
    pub fn integer(ident: String) -> Self {
        Declaration::Integer(ident)
    }
}

impl ToString for Declaration {
    /// Generates verilog code.
    fn to_string(&self) -> String {
        match self {
            Self::Net(shape, ident) => match shape.dim() {
                2 => {
                    assert!(!shape.is_signed());
                    format!("wire [{}-1:0] {}[{}-1:0];", shape.get(1), ident, shape.get(0))
                }
                1 => {
                    let width = shape.width();
                    if width > 1 {
                        match shape.is_signed() {
                            true => format!("wire signed [{}-1:0] {};", width, ident),
                            false => format!("wire [{}-1:0] {};", width, ident),
                        }
                    } else {
                        match shape.is_signed() {
                            true => format!("wire signed {};", ident),
                            false => format!("wire {};", ident),
                        }
                    }
                }
                _ => unimplemented!(),
            },
            Self::Reg(shape, ident, Some(expr)) => {
                assert_eq!(shape.dim(), 1);
                let width = shape.width();
                if width > 1 {
                    match shape.is_signed() {
                        true => {
                            format!("reg signed [{}-1:0] {} = {};", width, ident, expr.to_string())
                        }
                        false => format!("reg [{}-1:0] {} = {};", width, ident, expr.to_string()),
                    }
                } else {
                    match shape.is_signed() {
                        true => format!("reg signed {} = {};", ident, expr.to_string()),
                        false => {
                            format!("reg {} = {};", ident, expr.to_string())
                        }
                    }
                }
            }
            Self::Reg(shape, ident, None) => match shape.dim() {
                2 => {
                    assert!(!shape.is_signed());
                    format!("reg [{}-1:0] {}[{}-1:0];", shape.get(1), ident, shape.get(0))
                }
                1 => {
                    let width = shape.width();
                    if width > 1 {
                        match shape.is_signed() {
                            true => {
                                format!("reg signed [{}-1:0] {};", width, ident)
                            }
                            false => format!("reg [{}-1:0] {};", width, ident),
                        }
                    } else {
                        match shape.is_signed() {
                            true => format!("reg signed {};", ident),
                            false => format!("reg {};", ident),
                        }
                    }
                }
                _ => unimplemented!(),
            },
            Self::Integer(ident) => format!("integer {};", ident),
        }
    }
}

/// Continuous assign.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ContinuousAssign(pub Expression, pub Expression);

/// Generates verilog code for continuous assigns.
pub fn gen_verilog_conts(conts: &[ContinuousAssign]) -> String {
    conts.iter().map(|cont| cont.to_string()).collect::<Vec<_>>().join("\n")
}

impl ToString for ContinuousAssign {
    fn to_string(&self) -> String {
        format!("assign {} = {};", self.0.to_string(), self.1.to_string())
    }
}

impl ContinuousAssign {
    /// Creates new continuous assign.
    pub fn new(lvalue: Expression, expr: Expression) -> Self {
        Self(lvalue, expr)
    }
}

/// Module instantiation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ModuleInstantiation {
    /// Module name.
    pub module_name: String,

    /// Inst name.
    pub inst_name: String,

    /// Params.
    pub params: Vec<(String, usize)>,

    /// Port connections.
    pub port_connections: Vec<(String, Expression)>,
}

impl ToString for ModuleInstantiation {
    fn to_string(&self) -> String {
        format!(
            "{} #(\n{}\n)\n{} (\n{}\n);",
            self.module_name,
            self.params
                .iter()
                .map(|(name, value)| { format!("    .{}({})", name, value) })
                .collect::<Vec<_>>()
                .join(",\n"),
            self.inst_name,
            self.port_connections
                .iter()
                .map(|(port_name, expr)| { format!("    .{}({})", port_name, expr.to_string()) })
                .collect::<Vec<_>>()
                .join(",\n")
        )
    }
}

impl ModuleInstantiation {
    /// Creates new module instantiation.
    pub fn new(
        module_name: String,
        inst_name: String,
        params: Vec<(String, usize)>,
        port_connections: Vec<(String, Expression)>,
    ) -> Self {
        Self { module_name, inst_name, params, port_connections }
    }
}

/// Statement.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Statement {
    /// Blocking assignment.
    BlockingAssignment(Expression, Expression, rustc_span::Span),

    /// Conditional statement.
    Conditional(Vec<(Expression, Vec<Statement>)>, Vec<Statement>, rustc_span::Span),

    /// Loop statement.
    Loop(String, Expression, Vec<Statement>, rustc_span::Span),

    /// Nonblocking assignment.
    NonblockingAssignment(Expression, Expression, rustc_span::Span),

    /// Case statement.
    Case(Expression, Vec<(Expression, Vec<Statement>)>, Vec<Statement>, rustc_span::Span),

    /// Display
    Display(String, Vec<Expression>, rustc_span::Span),

    /// Fatal
    Fatal,
}

impl Statement {
    /// Blocking assignment.
    #[inline]
    pub fn blocking_assignment(lvalue: Expression, expr: Expression, span: rustc_span::Span) -> Self {
        assert!(
            matches!(lvalue, Expression::Primary(Primary::HierarchicalIdentifier(_, _))),
            "lvalue should be hierarchical identifier"
        );
        Statement::BlockingAssignment(lvalue, expr, span)
    }

    /// Nonblocking assignment.
    #[inline]
    pub fn nonblocking_assignment(lvalue: Expression, expr: Expression, span: rustc_span::Span) -> Self {
        assert!(
            matches!(lvalue, Expression::Primary(Primary::HierarchicalIdentifier(_, _))),
            "lvalue should be hierarchical identifier"
        );
        Statement::NonblockingAssignment(lvalue, expr, span)
    }
}

impl ToString for Statement {
    fn to_string(&self) -> String {
        match self {
            Self::BlockingAssignment(lvalue, expr, span) => {
                format!("{} = {}; // {span:?}", lvalue.to_string(), expr.to_string(),)
            }
            Self::Conditional(cond_expr_pairs, else_stmt, span) if else_stmt.is_empty() => {
                let conditional = cond_expr_pairs
                    .iter()
                    .map(|(cond, expr)| {
                        format!(
                            "if ({}) begin\n{}\nend",
                            cond.to_string(),
                            indent(expr.iter().map(|stmt| stmt.to_string()).collect::<Vec<_>>().join("\n"), INDENT),
                        )
                    })
                    .join("\nelse ");

                format!("// {span:?}\n{conditional}")
            }
            Self::Conditional(cond_expr_pairs, else_stmt, span) => {
                assert!(!cond_expr_pairs.is_empty());
                let conditional = cond_expr_pairs
                    .iter()
                    .map(|(cond, expr)| {
                        format!(
                            "if ({}) begin\n{}\nend",
                            cond.to_string(),
                            indent(expr.iter().map(|stmt| stmt.to_string()).collect::<Vec<_>>().join("\n"), INDENT),
                        )
                    })
                    .join("\nelse ");
                let else_stmt =
                    indent(else_stmt.iter().map(|stmt| stmt.to_string()).collect::<Vec<_>>().join("\n"), INDENT);
                format!("// {span:?}\n{conditional}\nelse begin\n{else_stmt}\nend",)
            }
            Self::Loop(ident, count, stmt, span) => {
                format!(
                    "// {span:?}\nfor ({} = 0; {} < {}; {} = {} + 1) begin\n{}\nend",
                    ident,
                    ident,
                    count.to_string(),
                    ident,
                    ident,
                    indent(stmt.iter().map(|stmt| stmt.to_string()).collect::<Vec<_>>().join("\n"), INDENT),
                )
            }
            Self::NonblockingAssignment(lvalue, expr, span) => {
                format!("{} <= {}; // {span:?}", lvalue.to_string(), expr.to_string(),)
            }
            Self::Case(case_expr, case_items, default, span) => {
                let case_items_code = case_items.iter().map(|(cond, stmt)| {
                    format!(
                        "{}: begin\n{}\nend",
                        cond.to_string(),
                        indent(stmt.iter().map(|stmt| stmt.to_string()).collect::<Vec<_>>().join("\n"), INDENT)
                    )
                });

                format!(
                    "// {span:?}\ncase ({})\n{}{}\nendcase",
                    case_expr.to_string(),
                    indent(case_items_code.collect::<Vec<_>>().join("\n"), INDENT),
                    if default.is_empty() {
                        "".to_string()
                    } else {
                        indent(
                            format!(
                                "\ndefault: begin\n{}\nend",
                                indent(
                                    default.iter().map(|stmt| stmt.to_string()).collect::<Vec<_>>().join("\n"),
                                    INDENT
                                ),
                            ),
                            INDENT,
                        )
                    }
                )
            }
            Self::Display(fstring, args, span) => {
                if args.is_empty() {
                    format!(
                        // NOTE: 32'h80000001 is `stdout`
                        "$fdisplay(32'h80000002,\"[%0t] {}\", $time); // {span:?}",
                        fstring
                    )
                } else {
                    format!(
                        // NOTE: 32'h80000001 is `stdout`
                        "$fdisplay(32'h80000002,\"[%0t] {}\", $time, {}); // {span:?}",
                        fstring,
                        args.iter().map(|arg| arg.to_string()).join(", ")
                    )
                }
            }
            Statement::Fatal => "$fatal;".to_string(),
        }
    }
}

/// Expression.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Expression {
    /// Primary.
    Primary(Primary),

    /// Unary expression.
    Unary(UnaryOp, Primary),

    /// Binary expression.
    Binary(Box<Expression>, BinaryOp, Box<Expression>),

    /// Conditional expression.
    Conditional(Box<Expression>, Box<Expression>, Box<Expression>),
}

/// Range.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Range {
    /// Index: `[index]`
    Index(Box<Expression>),

    /// Range: `[base +: offset]`
    Range(Box<Expression>, Box<Expression>),
}

/// Primary.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Primary {
    /// Number.
    Number(String),

    /// Hierarchical identifier.
    // TODO: Add range expression
    HierarchicalIdentifier(String, Option<Range>),

    /// Concatenation.
    Concatenation(Concatenation),

    /// Multiple concatenation.
    // TODO: Add constant expression
    MultipleConcatenation(usize, Concatenation),

    /// Mintypmax expression.
    MintypmaxExpression(Box<Expression>),
}

/// Concatenation.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Concatenation {
    /// Expressions.
    pub exprs: Vec<Expression>,
}

impl ToString for Expression {
    fn to_string(&self) -> String {
        match self {
            Self::Primary(prim) => prim.to_string(),
            Self::Unary(op, prim) => {
                format!("{}{}", op.to_string(), prim.to_string())
            }
            Self::Binary(lhs, op, rhs) => {
                format!("{} {} {}", lhs.to_string(), op.to_string(), rhs.to_string())
            }
            Self::Conditional(cond, then_expr, else_expr) => {
                format!("{} ? {} : {}", cond.to_string(), then_expr.to_string(), else_expr.to_string(),)
            }
        }
    }
}

impl From<String> for Expression {
    fn from(ident: String) -> Self {
        Expression::ident(ident)
    }
}

impl Expression {
    /// Number.
    pub fn number(num: String) -> Self {
        Self::Primary(Primary::Number(num))
    }

    /// Identifier.
    pub fn ident(ident: String) -> Self {
        Self::Primary(Primary::HierarchicalIdentifier(ident, None))
    }

    /// TODO: Documentation
    pub fn with_range(self, range: Range) -> Self {
        if let Expression::Primary(Primary::HierarchicalIdentifier(ident, None)) = self {
            Expression::Primary(Primary::HierarchicalIdentifier(ident, Some(range)))
        } else {
            todo!("with_range: self is not an identifier")
        }
    }

    /// Concatenation.
    pub fn concat(self, rhs: Expression) -> Self {
        Self::Primary(Primary::Concatenation(Concatenation { exprs: vec![self, rhs] }))
    }

    /// Multiple concatenation.
    pub fn multiple_concat(self, count: usize) -> Self {
        Self::Primary(Primary::MultipleConcatenation(
            count,
            if let Self::Primary(Primary::Concatenation(concat)) = self {
                concat
            } else {
                Concatenation { exprs: vec![self] }
            },
        ))
    }

    /// Mintypmax expression.
    pub fn mintypmax_expr(expr: Expression) -> Self {
        Self::Primary(Primary::MintypmaxExpression(Box::new(expr)))
    }

    /// Unary operation.
    pub fn unary(op: UnaryOp, expr: Self) -> Self {
        Self::Unary(
            op,
            if let Self::Primary(prim) = expr { prim } else { Primary::MintypmaxExpression(Box::new(expr)) },
        )
    }

    /// Binary operation.
    pub fn binary(op: BinaryOp, mut lhs: Expression, mut rhs: Expression) -> Self {
        // Operands of binary operation should be primary.
        if !lhs.is_primary() {
            lhs = Expression::mintypmax_expr(lhs);
        }

        if !rhs.is_primary() {
            rhs = Expression::mintypmax_expr(rhs);
        }

        Self::Binary(Box::new(lhs), op, Box::new(rhs))
    }

    /// Conditional expression.
    pub fn conditional(cond: Expression, then_expr: Expression, else_expr: Expression) -> Self {
        Self::Conditional(Box::new(cond), Box::new(then_expr), Box::new(else_expr))
    }

    /// Returns `true` if the expression is primary.
    pub fn is_primary(&self) -> bool {
        matches!(self, Self::Primary(_))
    }

    /// Returns `true` if the expression is identifier.
    pub fn is_identifier(&self) -> bool {
        matches!(self, Self::Primary(Primary::HierarchicalIdentifier(_, None)))
    }

    /// Converts into identifier string.
    pub fn into_ident(&self) -> Option<String> {
        if let Self::Primary(Primary::HierarchicalIdentifier(ident, None)) = self {
            Some(ident.clone())
        } else {
            None
        }
    }

    /// Converts into primary.
    #[must_use]
    pub fn into_primary(self) -> Self {
        if self.is_primary() {
            self
        } else {
            Self::mintypmax_expr(self)
        }
    }

    /// Returns `true` if the expression is concatenation.
    pub fn is_concat(&self) -> bool {
        matches!(self, Self::Primary(Primary::Concatenation(_)))
    }

    /// Returns `true` if the expression is a `don't-care`.
    pub fn is_x(&self) -> bool {
        match self {
            Expression::Primary(Primary::Number(n)) => {
                let split = n.split("'b").collect::<Vec<_>>();
                if split.len() == 2 {
                    split[1].chars().all(|c| c == 'x')
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

impl ToString for Range {
    fn to_string(&self) -> String {
        match self {
            Self::Index(index) => index.to_string(),
            Self::Range(base, offset) => {
                format!("{} +: {}", base.to_string(), offset.to_string())
            }
        }
    }
}

impl Range {
    /// Creates new index.
    pub fn new_index(index: Expression) -> Self {
        Self::Index(Box::new(index))
    }

    /// Creates new range.
    pub fn new_range(base: Expression, offset: Expression) -> Self {
        Self::Range(Box::new(base), Box::new(offset))
    }
}

impl ToString for Primary {
    fn to_string(&self) -> String {
        match self {
            Self::Number(num) => num.clone(),
            Self::HierarchicalIdentifier(ident, Some(range)) => {
                format!("{}[{}]", ident.clone(), range.to_string())
            }
            Self::HierarchicalIdentifier(ident, None) => ident.clone(),
            Self::Concatenation(concat) => concat.to_string(),
            Self::MultipleConcatenation(count, concat) => {
                format!("{{{}{}}}", count, concat.to_string())
            }
            Self::MintypmaxExpression(expr) => {
                format!("({})", expr.to_string())
            }
        }
    }
}

impl ToString for Concatenation {
    fn to_string(&self) -> String {
        assert!(!self.exprs.is_empty());
        format!("{{{}}}", self.exprs.iter().map(|expr| expr.to_string()).collect::<Vec<_>>().join(", "))
    }
}
