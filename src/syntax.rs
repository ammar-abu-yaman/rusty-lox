use std::{cell::RefCell, fmt::Display, rc::Rc};

use crate::{class::Class, function::CallableVariant, instance::Instance, token::Token};

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
        height: Option<usize>,
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
    Literal(Value),
    Variable {
        name: Token,
        height: Option<usize>,
    },
    LogicalOr{
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
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    Number(f64),
    String(String),
    Callable(CallableVariant),
    Instance(Rc<RefCell<Instance>>),
    Bool(bool),
    Nil,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Nil => write!(f, "nil"),
            Value::Callable(callable) => write!(f, "{callable}"),
            Value::Instance(instance) => write!(f, "{}", instance.borrow()),
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

    pub fn variable(name: Token, height: Option<usize>) -> Self {
        Self::Variable { name, height }
    }

    pub fn assign(name: Token, value: Expr) -> Self {
        Self::Asign {
            name,
            value: BoxedExpr::new(value),
            height: None,
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
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Asign { name: Token { lexeme, .. }, value, .. } => {
                                write!(f, "(= {lexeme} {value})")
                            },
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
            Expr::Literal(Value::Bool(b)) => write!(f, "{b}"),
            Expr::Literal(Value::String(s)) => write!(f, "{s}"),
            Expr::Literal(Value::Nil) => write!(f, "nil"),
            Expr::Literal(Value::Number(n)) => write!(f, "{n:?}"),
            Expr::Variable { name: Token { lexeme, .. }, .. } => write!(f, "{lexeme}"),
            Expr::LogicalOr { left, right } => write!(f, "(or {left} {right})"),
            Expr::Literal(value) => write!(f, "{value}"),
            Expr::LogicalAnd { left, right } => write!(f, "(and {left} {right})"),
            Expr::Call { callee, args, ..  } => {
                                write!(f, "(call {callee} ")?;
                                if !args.is_empty() {
                                    write!(f, "{}", args[0])?;
                                    for arg in args.iter().skip(1) {
                                        write!(f, ", {arg}")?;
                                    }
                                }
                                write!(f, ")")
                            }
            Expr::Get { object, name: Token { lexeme, ..} } => write!(f, "(get {object} {lexeme})"),
            Expr::Set { object, name: Token { lexeme, ..}, value } => write!(f, "(set {object} {lexeme} {value})"),
                            
        }
    }
}
