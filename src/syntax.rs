use std::cell::{Cell, RefCell};
use std::fmt::Display;
use std::rc::Rc;

use crate::class::Class;
use crate::function::{Function, NativeFunction};
use crate::instance::Instance;
use crate::token::Token;

pub type BoxedExpr = Box<Expr>;
pub type BoxedStatement = Box<Statement>;

#[derive(Debug, Clone)]
pub enum Statement {
    FunDecl(FunctionDecl),
    VarDecl(VariableDecl),
    ClassDecl(ClassDecl),
    Print(PrintStatement),
    Expr(ExpressionStatement),
    Block(BlockStatement),
    If(IfStatemnet),
    While(WhileStatement),
    Return(ReturnStatement),
}

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: Token,
    pub superclass: Option<Expr>,
    pub methods: Vec<FunctionDecl>,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: Token,
    pub params: Vec<Token>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct VariableDecl {
    pub name: Token,
    pub initializer: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct PrintStatement {
    pub print_token: Token,
    pub expr: Expr,
}

#[derive(Debug, Clone)]
pub struct BlockStatement {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct IfStatemnet {
    pub condition: Expr,
    pub if_branch: BoxedStatement,
    pub else_branch: Option<BoxedStatement>,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub return_token: Token,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct WhileStatement {
    pub condition: Expr,
    pub body: BoxedStatement,
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement {
    pub expr: Expr,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Asign {
        name: Token,
        height: Cell<Option<usize>>,
        value: BoxedExpr,
    },
    Binary {
        left: BoxedExpr,
        operator: Token,
        right: BoxedExpr,
    },
    Unary {
        operator: Token,
        expr: BoxedExpr,
    },
    Grouping(BoxedExpr),
    Literal(Literal),
    Variable {
        name: Token,
        height: Cell<Option<usize>>,
    },
    LogicalOr {
        left: BoxedExpr,
        right: BoxedExpr,
    },
    LogicalAnd {
        left: BoxedExpr,
        right: BoxedExpr,
    },
    Call {
        callee: BoxedExpr,
        paren: Token,
        args: Vec<Expr>,
    },
    Get {
        object: BoxedExpr,
        name: Token,
    },
    Set {
        object: BoxedExpr,
        name: Token,
        value: BoxedExpr,
    },
    This {
        keyword: Token,
        height: Cell<Option<usize>>,
    },
    Super {
        keyword: Token,
        method: Token,
        height: Cell<Option<usize>>,
    },
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value<'a> {
    Number(f64),
    String(String),
    Class(Rc<Class<'a>>),
    Function(Rc<Function<'a>>),
    NativeFunction(Rc<NativeFunction>),
    Instance(Rc<RefCell<Instance<'a>>>),
    Bool(bool),
    Nil,
}

impl From<&Literal> for Value<'_> {
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
pub enum Literal {
    Number(f64),
    String(String),
    Bool(bool),
    Nil,
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "{n}"),
            Literal::String(s) => write!(f, "{s}"),
            Literal::Bool(b) => write!(f, "{b}"),
            Literal::Nil => write!(f, "nil"),
        }
    }
}

impl Display for Value<'_> {
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

impl Expr {
    pub fn grouping(expr: Expr) -> Self {
        Self::Grouping(BoxedExpr::new(expr))
    }

    pub fn unary(operator: Token, expr: Expr) -> Self {
        Self::Unary {
            operator,
            expr: BoxedExpr::new(expr),
        }
    }

    pub fn binary(left: Expr, operator: Token, right: Expr) -> Self {
        Self::Binary {
            left: BoxedExpr::new(left),
            operator,
            right: BoxedExpr::new(right),
        }
    }

    pub fn variable(name: Token, height: Cell<Option<usize>>) -> Self {
        Self::Variable { name, height }
    }

    pub fn assign(name: Token, value: Expr) -> Self {
        Self::Asign {
            name,
            value: BoxedExpr::new(value),
            height: Cell::new(None),
        }
    }

    pub fn or(left: Expr, right: Expr) -> Self {
        Self::LogicalOr {
            left: BoxedExpr::new(left),
            right: BoxedExpr::new(right),
        }
    }

    pub fn and(left: Expr, right: Expr) -> Self {
        Self::LogicalAnd {
            left: BoxedExpr::new(left),
            right: BoxedExpr::new(right),
        }
    }

    pub fn call(callee: Expr, paren: Token, args: Vec<Expr>) -> Self {
        Self::Call {
            callee: BoxedExpr::new(callee),
            paren,
            args,
        }
    }

    pub fn get(object: Expr, name: Token) -> Self {
        Self::Get {
            object: BoxedExpr::new(object),
            name,
        }
    }

    pub fn set(object: BoxedExpr, name: Token, value: Expr) -> Self {
        Self::Set {
            object,
            name,
            value: BoxedExpr::new(value),
        }
    }

    pub fn this(keyword: Token) -> Self {
        Self::This {
            keyword,
            height: Cell::new(None),
        }
    }

    pub fn literal(literal: Literal) -> Self {
        Self::Literal(literal)
    }

    pub fn super_(keyword: Token, method: Token) -> Self {
        Self::Super {
            keyword,
            method,
            height: Cell::new(None),
        }
    }
}

impl Display for Expr {
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