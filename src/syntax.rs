use std::cell::{Cell, RefCell};
use std::fmt::Display;
use std::rc::Rc;

use crate::interpreter::tree_walker::class::Class;
use crate::interpreter::tree_walker::function::{Function, NativeFunction};
use crate::interpreter::tree_walker::instance::Instance;
use crate::token::Token;

pub type BoxedExpr<'t> = Box<Expr<'t>>;
pub type BoxedStatement<'t> = Box<Statement<'t>>;

#[derive(Debug, Clone)]
pub enum Statement<'t> {
    FunDecl(FunctionDecl<'t>),
    VarDecl(VariableDecl<'t>),
    ClassDecl(ClassDecl<'t>),
    Print(PrintStatement<'t>),
    Expr(ExpressionStatement<'t>),
    Block(BlockStatement<'t>),
    If(IfStatemnet<'t>),
    While(WhileStatement<'t>),
    Return(ReturnStatement<'t>),
}

#[derive(Debug, Clone)]
pub struct ClassDecl<'t> {
    pub name: Token<'t>,
    pub superclass: Option<Expr<'t>>,
    pub methods: Vec<FunctionDecl<'t>>,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl<'t> {
    pub name: Token<'t>,
    pub params: Vec<Token<'t>>,
    pub body: Vec<Statement<'t>>,
}

#[derive(Debug, Clone)]
pub struct VariableDecl<'t> {
    pub name: Token<'t>,
    pub initializer: Option<Expr<'t>>,
}

#[derive(Debug, Clone)]
pub struct PrintStatement<'t> {
    pub print_token: Token<'t>,
    pub expr: Expr<'t>,
}

#[derive(Debug, Clone)]
pub struct BlockStatement<'t> {
    pub statements: Vec<Statement<'t>>,
}

#[derive(Debug, Clone)]
pub struct IfStatemnet<'t> {
    pub condition: Expr<'t>,
    pub if_branch: BoxedStatement<'t>,
    pub else_branch: Option<BoxedStatement<'t>>,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement<'t> {
    pub return_token: Token<'t>,
    pub value: Option<Expr<'t>>,
}

#[derive(Debug, Clone)]
pub struct WhileStatement<'t> {
    pub condition: Expr<'t>,
    pub body: BoxedStatement<'t>,
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement<'t> {
    pub expr: Expr<'t>,
}

#[derive(Debug, Clone)]
pub enum Expr<'t> {
    Asign {
        name: Token<'t>,
        height: Cell<Option<usize>>,
        value: BoxedExpr<'t>,
    },
    Binary {
        left: BoxedExpr<'t>,
        operator: Token<'t>,
        right: BoxedExpr<'t>,
    },
    Unary {
        operator: Token<'t>,
        expr: BoxedExpr<'t>,
    },
    Grouping(BoxedExpr<'t>),
    Literal(Literal<'t>),
    Variable {
        name: Token<'t>,
        height: Cell<Option<usize>>,
    },
    LogicalOr {
        left: BoxedExpr<'t>,
        right: BoxedExpr<'t>,
    },
    LogicalAnd {
        left: BoxedExpr<'t>,
        right: BoxedExpr<'t>,
    },
    Call {
        callee: BoxedExpr<'t>,
        paren: Token<'t>,
        args: Vec<Expr<'t>>,
    },
    Get {
        object: BoxedExpr<'t>,
        name: Token<'t>,
    },
    Set {
        object: BoxedExpr<'t>,
        name: Token<'t>,
        value: BoxedExpr<'t>,
    },
    This {
        keyword: Token<'t>,
        height: Cell<Option<usize>>,
    },
    Super {
        keyword: Token<'t>,
        method: Token<'t>,
        height: Cell<Option<usize>>,
    },
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value<'a, 't> {
    Number(f64),
    String(String),
    Class(Rc<Class<'a, 't>>),
    Function(Rc<Function<'a, 't>>),
    NativeFunction(Rc<NativeFunction<'t, 'a>>),
    Instance(Rc<RefCell<Instance<'a, 't>>>),
    Bool(bool),
    Nil,
}

impl<'a> From<&Literal<'a>> for Value<'_, '_> {
    fn from(value: &Literal) -> Self {
        match value {
            Literal::Number(n) => Value::Number(*n),
            Literal::String(_) => Value::String(value.to_string()),
            Literal::Bool(_) => Value::Bool(value.to_string().parse().unwrap()),
            Literal::Nil => Value::Nil,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Literal<'t> {
    Number(f64),
    String(&'t str),
    Bool(bool),
    Nil,
}

impl Display for Literal<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "{n}"),
            Literal::String(s) => write!(f, "{s}"),
            Literal::Bool(b) => write!(f, "{b}"),
            Literal::Nil => write!(f, "nil"),
        }
    }
}

impl Display for Value<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Nil => write!(f, "nil"),
            Value::Class(class) => write!(f, "{class}"),
            Value::Instance(instance) => write!(f, "{}", instance.borrow()),
            Value::Function(function) => write!(f, "{function}"),
            Value::NativeFunction(native_function) => write!(f, "{native_function}"),
        }
    }
}

impl<'t> Expr<'t> {
    pub fn grouping(expr: Expr<'t>) -> Self {
        Self::Grouping(BoxedExpr::new(expr))
    }

    pub fn unary(operator: Token<'t>, expr: Expr<'t>) -> Self {
        Self::Unary {
            operator,
            expr: BoxedExpr::new(expr),
        }
    }

    pub fn binary(left: Expr<'t>, operator: Token<'t>, right: Expr<'t>) -> Self {
        Self::Binary {
            left: BoxedExpr::new(left),
            operator,
            right: BoxedExpr::new(right),
        }
    }

    pub fn variable(name: Token<'t>, height: Cell<Option<usize>>) -> Self {
        Self::Variable { name, height }
    }

    pub fn assign(name: Token<'t>, value: Expr<'t>) -> Self {
        Self::Asign {
            name,
            value: BoxedExpr::new(value),
            height: Cell::new(None),
        }
    }

    pub fn or(left: Expr<'t>, right: Expr<'t>) -> Self {
        Self::LogicalOr {
            left: BoxedExpr::new(left),
            right: BoxedExpr::new(right),
        }
    }

    pub fn and(left: Expr<'t>, right: Expr<'t>) -> Self {
        Self::LogicalAnd {
            left: BoxedExpr::new(left),
            right: BoxedExpr::new(right),
        }
    }

    pub fn call(callee: Expr<'t>, paren: Token<'t>, args: Vec<Expr<'t>>) -> Self {
        Self::Call {
            callee: BoxedExpr::new(callee),
            paren,
            args,
        }
    }

    pub fn get(object: Expr<'t>, name: Token<'t>) -> Self {
        Self::Get {
            object: BoxedExpr::new(object),
            name,
        }
    }

    pub fn set(object: BoxedExpr<'t>, name: Token<'t>, value: Expr<'t>) -> Self {
        Self::Set {
            object,
            name,
            value: BoxedExpr::new(value),
        }
    }

    pub fn this(keyword: Token<'t>) -> Self {
        Self::This {
            keyword,
            height: Cell::new(None),
        }
    }

    pub fn literal(literal: Literal<'t>) -> Self {
        Self::Literal(literal)
    }

    pub fn super_(keyword: Token<'t>, method: Token<'t>) -> Self {
        Self::Super {
            keyword,
            method,
            height: Cell::new(None),
        }
    }
}

impl Display for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Asign {
                name: Token { lexeme, .. },
                value,
                ..
            } => write!(f, "(= {lexeme} {value})"),
            Expr::Binary {
                left,
                operator: Token { lexeme, .. },
                right,
            } => write!(f, "({lexeme} {left} {right})"),
            Expr::Unary {
                operator: Token { lexeme, .. },
                expr,
            } => write!(f, "({lexeme} {expr})"),
            Expr::Grouping(expr) => write!(f, "(group {expr})"),
            Expr::Literal(Literal::Bool(b)) => write!(f, "{b}"),
            Expr::Literal(Literal::String(s)) => write!(f, "{s}"),
            Expr::Literal(Literal::Nil) => write!(f, "nil"),
            Expr::Literal(Literal::Number(n)) => write!(f, "{n:?}"),
            Expr::Variable {
                name: Token { lexeme, .. }, ..
            } => write!(f, "{lexeme}"),
            Expr::LogicalOr { left, right } => write!(f, "(or {left} {right})"),
            Expr::LogicalAnd { left, right } => write!(f, "(and {left} {right})"),
            Expr::Call { callee, args, .. } => {
                write!(f, "(call {callee} ")?;
                if !args.is_empty() {
                    write!(f, "{}", args[0])?;
                    for arg in args.iter().skip(1) {
                        write!(f, ", {arg}")?;
                    }
                }
                write!(f, ")")
            },
            Expr::Get {
                object,
                name: Token { lexeme, .. },
            } => write!(f, "(get {object} {lexeme})"),
            Expr::Set {
                object,
                name: Token { lexeme, .. },
                value,
            } => write!(f, "(set {object} {lexeme} {value})"),
            Expr::This { .. } => write!(f, "this"),
            Expr::Super {
                method: Token { lexeme, .. }, ..
            } => write!(f, "(super {lexeme})"),
        }
    }
}
