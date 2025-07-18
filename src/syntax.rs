use std::cell::{Cell, RefCell};
use std::fmt::Display;
use std::rc::Rc;

use crate::class::Class;
use crate::function::{Function, NativeFunction};
use crate::instance::Instance;
use crate::token::Token;

pub type BoxedExpr<'a> = Box<Expr<'a>>;
pub type BoxedStatement<'a> = Box<Statement<'a>>;

#[derive(Debug, Clone)]
pub enum Statement<'a> {
    FunDecl(FunctionDecl<'a>),
    VarDecl(VariableDecl<'a>),
    ClassDecl(ClassDecl<'a>),
    Print(PrintStatement<'a>),
    Expr(ExpressionStatement<'a>),
    Block(BlockStatement<'a>),
    If(IfStatemnet<'a>),
    While(WhileStatement<'a>),
    Return(ReturnStatement<'a>),
}

#[derive(Debug, Clone)]
pub struct ClassDecl<'a> {
    pub name: Token<'a>,
    pub superclass: Option<Expr<'a>>,
    pub methods: Vec<FunctionDecl<'a>>,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl<'a> {
    pub name: Token<'a>,
    pub params: Vec<Token<'a>>,
    pub body: Vec<Statement<'a>>,
}

#[derive(Debug, Clone)]
pub struct VariableDecl<'a> {
    pub name: Token<'a>,
    pub initializer: Option<Expr<'a>>,
}

#[derive(Debug, Clone)]
pub struct PrintStatement<'a> {
    pub print_token: Token<'a>,
    pub expr: Expr<'a>,
}

#[derive(Debug, Clone)]
pub struct BlockStatement<'a> {
    pub statements: Vec<Statement<'a>>,
}

#[derive(Debug, Clone)]
pub struct IfStatemnet<'a> {
    pub condition: Expr<'a>,
    pub if_branch: BoxedStatement<'a>,
    pub else_branch: Option<BoxedStatement<'a>>,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement<'a> {
    pub return_token: Token<'a>,
    pub value: Option<Expr<'a>>,
}

#[derive(Debug, Clone)]
pub struct WhileStatement<'a> {
    pub condition: Expr<'a>,
    pub body: BoxedStatement<'a>,
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement<'a> {
    pub expr: Expr<'a>,
}

#[derive(Debug, Clone)]
pub enum Expr<'a> {
    Asign {
        name: Token<'a>,
        height: Cell<Option<usize>>,
        value: BoxedExpr<'a>,
    },
    Binary {
        left: BoxedExpr<'a>,
        operator: Token<'a>,
        right: BoxedExpr<'a>,
    },
    Unary {
        operator: Token<'a>,
        expr: BoxedExpr<'a>,
    },
    Grouping(BoxedExpr<'a>),
    Literal(Literal<'a>),
    Variable {
        name: Token<'a>,
        height: Cell<Option<usize>>,
    },
    LogicalOr {
        left: BoxedExpr<'a>,
        right: BoxedExpr<'a>,
    },
    LogicalAnd {
        left: BoxedExpr<'a>,
        right: BoxedExpr<'a>,
    },
    Call {
        callee: BoxedExpr<'a>,
        paren: Token<'a>,
        args: Vec<Expr<'a>>,
    },
    Get {
        object: BoxedExpr<'a>,
        name: Token<'a>,
    },
    Set {
        object: BoxedExpr<'a>,
        name: Token<'a>,
        value: BoxedExpr<'a>,
    },
    This {
        keyword: Token<'a>,
        height: Cell<Option<usize>>,
    },
    Super {
        keyword: Token<'a>,
        method: Token<'a>,
        height: Cell<Option<usize>>,
    },
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value<'a, 't> {
    Number(f64),
    String(String),
    Class(Rc<Class<'a>>),
    Function(Rc<Function<'a, 't>>),
    NativeFunction(Rc<NativeFunction<'t, 'a>>),
    Instance(Rc<RefCell<Instance<'a>>>),
    Bool(bool),
    Nil,
}

impl <'a> From<&Literal<'a>> for Value<'_, '_> {
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
pub enum Literal<'a> {
    Number(f64),
    String(&'a str),
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

impl <'a> Expr<'a> {
    pub fn grouping(expr: Expr<'a>) -> Self {
        Self::Grouping(BoxedExpr::new(expr))
    }

    pub fn unary(operator: Token<'a>, expr: Expr<'a>) -> Self {
        Self::Unary {
            operator,
            expr: BoxedExpr::new(expr),
        }
    }

    pub fn binary(left: Expr<'a>, operator: Token<'a>, right: Expr<'a>) -> Self {
        Self::Binary {
            left: BoxedExpr::new(left),
            operator,
            right: BoxedExpr::new(right),
        }
    }

    pub fn variable(name: Token<'a>, height: Cell<Option<usize>>) -> Self {
        Self::Variable { name, height }
    }

    pub fn assign(name: Token<'a>, value: Expr<'a>) -> Self {
        Self::Asign {
            name,
            value: BoxedExpr::new(value),
            height: Cell::new(None),
        }
    }

    pub fn or(left: Expr<'a>, right: Expr<'a>) -> Self {
        Self::LogicalOr {
            left: BoxedExpr::new(left),
            right: BoxedExpr::new(right),
        }
    }

    pub fn and(left: Expr<'a>, right: Expr<'a>) -> Self {
        Self::LogicalAnd {
            left: BoxedExpr::new(left),
            right: BoxedExpr::new(right),
        }
    }

    pub fn call(callee: Expr<'a>, paren: Token<'a>, args: Vec<Expr<'a>>) -> Self {
        Self::Call {
            callee: BoxedExpr::new(callee),
            paren,
            args,
        }
    }

    pub fn get(object: Expr<'a>, name: Token<'a>) -> Self {
        Self::Get {
            object: BoxedExpr::new(object),
            name,
        }
    }

    pub fn set(object: BoxedExpr<'a>, name: Token<'a>, value: Expr<'a>) -> Self {
        Self::Set {
            object,
            name,
            value: BoxedExpr::new(value),
        }
    }

    pub fn this(keyword: Token<'a>) -> Self {
        Self::This {
            keyword,
            height: Cell::new(None),
        }
    }

    pub fn literal(literal: Literal<'a>) -> Self {
        Self::Literal(literal)
    }

    pub fn super_(keyword: Token<'a>, method: Token<'a>) -> Self {
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