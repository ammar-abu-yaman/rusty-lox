use super::instruction::OpCode;
use crate::token::TokenType;

use super::log::Logger;
use super::Scanner;
use crate::token::Token;

use super::mem::MemoryManager;
use super::Chunk;
use super::Value;
use super::STACK_SIZE;

use arrayvec::ArrayVec;

#[derive(Debug, Clone)]
struct Local<'a> {
    token: Token<'a>,
    depth: i32,
}

#[derive(Debug, Default)]
struct Stack<'a> {
    locals: ArrayVec<Local<'a>, STACK_SIZE>,
    depth: i32,
}

pub struct ByteCodeCompiler<'a, W: std::io::Write> {
    scanner: &'a Scanner,
    pub has_error: bool,
    pub panic_mode: bool,
    logger: &'a mut Logger<W>,
    chunk: &'a mut Chunk,
    mem: &'a mut MemoryManager,
    stack: Stack<'a>,
    previous: Option<Token<'a>>,
    current: Option<Token<'a>>,
}

impl<'a, W: std::io::Write> ByteCodeCompiler<'a, W> {
    pub fn new(scanner: &'a Scanner, logger: &'a mut Logger<W>, chunk: &'a mut Chunk, mem: &'a mut MemoryManager) -> Self {
        Self {
            scanner,
            logger,
            chunk,
            mem,
            has_error: false,
            panic_mode: false,
            stack: Stack::default(),
            previous: None,
            current: None,
        }
    }
}
impl<'a, W: std::io::Write> ByteCodeCompiler<'a, W> {
    pub fn compile(&mut self) {
        self.advance();
        while !self.check(TokenType::Eof) {
            self.declaration();
        }
        self.consume(TokenType::Eof, "Expected EOF");
    }
}

impl<'a, W: std::io::Write> ByteCodeCompiler<'a, W> {
    fn declaration(&mut self) {
        if self.match_current(TokenType::Var) {
            self.var_declaration();
        } else if self.match_current(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.statement();
        }

        if self.panic_mode {
            self.synchronize();
        }
    }

    fn var_declaration(&mut self) {
        let global_var_index = self.parse_var("Expected variable name");
        if self.match_current(TokenType::Asign) {
            self.expression();
        } else {
            self.write_bytes([OpCode::LoadNil]);
        }

        self.consume(TokenType::SemiColon, "Expected ';' after variable declaration");

        self.define_var(global_var_index);
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;
        while !self.check(TokenType::Eof) && self.current.is_some() {
            use TokenType::*;
            if matches!(self.previous.as_ref(), Some(token) if token.token_type == SemiColon) {
                return;
            }
            match self.current.as_ref().unwrap().token_type {
                Var | Class | Fun | For | If | While | Return | Print => return,
                _ => {},
            }

            self.advance();
        }
    }

    fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.declaration();
        }
        self.consume(TokenType::RightBrace, "Expected '}' after block");
    }

    fn define_var(&mut self, global_var_index: u8) {
        if self.stack.depth > 0 {
            self.mark_initialized();
            return;
        }
        self.write_bytes([OpCode::DefineGlobal as u8, global_var_index]);
    }
}

impl<'a, W: std::io::Write> ByteCodeCompiler<'a, W> {
    fn statement(&mut self) {
        use TokenType::Print;
        if (self.match_current(Print)) {
            self.print_statement();
        } else {
            self.expression_statement();
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::SemiColon, "Expected ';' after expression");
        self.write_bytes([OpCode::Print]);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::SemiColon, "Expected ';' after expression");
        self.write_bytes([OpCode::Pop]);
    }
}

impl<'a, W: std::io::Write> ByteCodeCompiler<'a, W> {
    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn number(&mut self, _can_assign: bool) {
        let value = Value::Number(self.previous.as_ref().unwrap().literal.as_number());
        self.emit_const(value);
    }

    fn grouping(&mut self, _can_assign: bool) {
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after expression");
    }

    fn unary(&mut self, _can_assign: bool) {
        let operator = self.previous.as_ref().unwrap().token_type;
        self.expression();

        match operator {
            TokenType::Minus => self.write_bytes([OpCode::Negate]),
            TokenType::Not => self.write_bytes([OpCode::Not]),
            _ => return,
        }
    }

    fn binary(&mut self, _can_assign: bool) {
        let operator = self.previous.as_ref().unwrap().token_type;
        let rule = ParseRule::<W>::derive_rule(operator);
        self.parse_precedence(rule.precedence.next());
        match operator {
            TokenType::Plus => self.write_bytes([OpCode::Add]),
            TokenType::Minus => self.write_bytes([OpCode::Subtract]),
            TokenType::Star => self.write_bytes([OpCode::Multiply]),
            TokenType::Div => self.write_bytes([OpCode::Divide]),
            TokenType::Equal => self.write_bytes([OpCode::Equal]),
            TokenType::NotEqual => self.write_bytes([OpCode::Equal, OpCode::Not]),
            TokenType::Greater => self.write_bytes([OpCode::Greater]),
            TokenType::GreaterEq => self.write_bytes([OpCode::Less, OpCode::Not]),
            TokenType::Less => self.write_bytes([OpCode::Less]),
            TokenType::LessEq => self.write_bytes([OpCode::Greater, OpCode::Not]),
            _ => unreachable!(""),
        }
    }

    fn literal(&mut self, _can_assign: bool) {
        let operator = self.previous.as_ref().unwrap().token_type;
        match operator {
            TokenType::True => self.write_bytes([OpCode::LoadTrue]),
            TokenType::False => self.write_bytes([OpCode::LoadFalse]),
            TokenType::Nil => self.write_bytes([OpCode::LoadNil]),
            _ => unreachable!(""),
        }
    }

    fn string(&mut self, _can_assign: bool) {
        let token_str = self.previous.as_ref().unwrap().literal.as_string();
        let str_object = self.mem.intern_string(token_str);
        self.emit_const(Value::Object(str_object));
    }

    fn variable(&mut self, can_assign: bool) {
        self.named_var(self.previous.clone().unwrap(), can_assign);
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let rule = ParseRule::<W>::derive_rule(self.previous.as_ref().unwrap().token_type);
        if rule.prefix.is_none() {
            self.error("Expected expression");
            return;
        }

        let can_assign = precedence <= Precedence::Assignment;
        rule.prefix.unwrap()(self, can_assign);

        while ParseRule::<W>::derive_rule(self.current.as_ref().unwrap().token_type).precedence >= precedence {
            self.advance();
            let rule = ParseRule::<W>::derive_rule(self.previous.as_ref().unwrap().token_type);
            rule.infix.unwrap()(self, can_assign);
        }

        if can_assign && self.match_current(TokenType::Asign) {
            self.error("Invalid assignment target");
        }
    }

    fn parse_var(&mut self, message: impl AsRef<str>) -> u8 {
        self.consume(TokenType::Identifier, message.as_ref());
        self.declare_var();
        if self.stack.depth > 0 {
            return 0;
        }
        return self.identifier_const(self.previous.clone().as_ref().unwrap());
    }

    fn named_var(&mut self, name: Token<'a>, can_assign: bool) {
        let (arg, get_op, set_op) = self.resolve_local(name.lexeme).map_or_else(
            || (self.identifier_const(&name), OpCode::GetGlobal as u8, OpCode::SetGlobal as u8),
            |i| (i as u8, OpCode::GetLocal as u8, OpCode::SetLocal as u8),
        );
        if can_assign && self.match_current(TokenType::Asign) {
            self.expression();
            self.write_bytes([set_op, arg]);
        } else {
            self.write_bytes([get_op, arg]);
        }
    }

    fn identifier_const(&mut self, token: &Token<'a>) -> u8 {
        let name_obj = self.mem.intern_string(token.lexeme);
        let name_val = Value::Object(name_obj);
        return self.add_const(name_val) as u8;
    }

    fn declare_var(&mut self) {
        if self.stack.depth == 0 {
            return;
        }
        let name = self.previous.clone().unwrap();

        for i in (0..self.stack.locals.len()).rev() {
            let local = &self.stack.locals[i];
            if local.depth != -1 && local.depth < self.stack.depth as i32 {
                break;
            }
            if name.lexeme == local.token.lexeme {
                self.error("Already a variable with this name in scope");
                return;
            }
        }

        self.add_local(name);
    }
}

impl<'a, W: std::io::Write> ByteCodeCompiler<'a, W> {
    fn consume(&mut self, token_type: TokenType, message: &str) {
        if !matches!(self.current.as_ref(), Some(token) if token.token_type == token_type) {
            self.error_at(&self.current.clone().unwrap(), message);
            return;
        }
        self.advance();
    }

    fn check(&self, token_type: TokenType) -> bool {
        matches!(self.current.as_ref(), Some(token) if token.token_type == token_type)
    }

    fn match_current(&mut self, token_type: TokenType) -> bool {
        if !self.check(token_type) {
            return false;
        }
        self.advance();
        true
    }

    fn advance(&mut self) {
        self.previous = self.current.take();
        loop {
            let token = self.scanner.next_token();
            if token.token_type == TokenType::Error {
                self.error_at(&token, token.error.as_deref().unwrap_or(""));
                continue;
            }
            self.current = Some(token);
            break;
        }
    }

    fn emit_const(&mut self, value: Value) {
        let const_index = self.add_const(value);
        self.write_bytes([OpCode::LoadConst as u8, const_index as u8]);
    }

    fn add_const(&mut self, value: Value) -> usize {
        self.chunk.constants.push(value);
        if self.chunk.constants.len() > u8::MAX.into() {
            self.error("Too many constants in chunk");
            return 0;
        }
        self.chunk.constants.len() - 1
    }

    fn add_local(&mut self, name: Token<'a>) {
        if self.stack.locals.len() >= u8::MAX.into() {
            self.error("Too many local variables in function");
            return;
        }
        let local = Local { token: name, depth: -1 };
        self.stack.locals.push(local);
    }

    fn mark_initialized(&mut self) {
        if let Some(local) = self.stack.locals.last_mut() {
            local.depth = self.stack.depth;
        }
    }

    fn resolve_local(&mut self, name: &str) -> Option<usize> {
        for (i, local) in self.stack.locals.iter().enumerate().rev() {
            if local.token.lexeme == name {
                if local.depth == -1 {
                    self.error("Can't read local variable in its own initializer.");
                }
                return Some(i);
            }
        }
        None
    }

    fn write_bytes<const N: usize, T: Into<u8>>(&mut self, bytes: [T; N]) {
        let line = self.previous.as_ref().unwrap().pos.line as u32;
        for byte in bytes {
            self.chunk.code.push(byte.into());
            self.chunk.lines.push(line);
        }
    }

    pub fn write_end(&mut self) {
        self.write_bytes([OpCode::Return]);
    }
}

impl<'a, W: std::io::Write> ByteCodeCompiler<'a, W> {
    fn begin_scope(&mut self) {
        self.stack.depth += 1;
    }

    fn end_scope(&mut self) {
        self.stack.depth -= 1;
        while !self.stack.locals.is_empty() && self.stack.locals.last().unwrap().depth > self.stack.depth {
            self.stack.locals.pop();
            self.write_bytes([OpCode::Pop]);
        }
    }
}

impl<'a, W: std::io::Write> ByteCodeCompiler<'a, W> {
    fn error(&mut self, message: &str) {
        self.error_at(&self.previous.clone().unwrap(), message);
    }

    fn error_at(&mut self, token: &Token<'_>, message: &str) {
        if self.panic_mode {
            return;
        }
        self.has_error = true;
        self.panic_mode = true;
        self.logger.log_err_at(token, message);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    None = 0,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
    TerminalPrecedence,
}

impl Precedence {
    pub fn next(self) -> Self {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::TerminalPrecedence,
            Precedence::TerminalPrecedence => Precedence::None,
        }
    }
}

type ParseFn<'a, W> = fn(&mut ByteCodeCompiler<'a, W>, bool);

#[derive(Clone, Copy)]
struct ParseRule<'a, W: std::io::Write> {
    prefix: Option<ParseFn<'a, W>>,
    infix: Option<ParseFn<'a, W>>,
    precedence: Precedence,
}

impl<'a, W: std::io::Write> ParseRule<'a, W> {
    fn new(prefix: Option<ParseFn<'a, W>>, infix: Option<ParseFn<'a, W>>, precedence: Precedence) -> Self {
        Self { prefix, infix, precedence }
    }

    fn derive_rule(token_type: TokenType) -> ParseRule<'a, W> {
        use TokenType::*;
        let rule = Self::new;
        let unary: Option<ParseFn<'a, W>> = Some(ByteCodeCompiler::unary);
        let binary: Option<ParseFn<'a, W>> = Some(ByteCodeCompiler::binary);
        let grouping: Option<ParseFn<'a, W>> = Some(ByteCodeCompiler::grouping);
        let literal: Option<ParseFn<'a, W>> = Some(ByteCodeCompiler::literal);
        let number: Option<ParseFn<'a, W>> = Some(ByteCodeCompiler::number);
        let string: Option<ParseFn<'a, W>> = Some(ByteCodeCompiler::string);
        let variable: Option<ParseFn<'a, W>> = Some(ByteCodeCompiler::variable);

        match token_type {
            LeftParen => rule(grouping, None, Precedence::None),
            RightParen => rule(None, None, Precedence::None),
            LeftBrace => rule(None, None, Precedence::None),
            RightBrace => rule(None, None, Precedence::None),
            Comma => rule(None, None, Precedence::None),
            Dot => rule(None, None, Precedence::None),
            Minus => rule(unary, binary, Precedence::Term),
            Plus => rule(None, binary, Precedence::Term),
            SemiColon => rule(None, None, Precedence::None),
            Div => rule(None, binary, Precedence::Factor),
            Star => rule(None, binary, Precedence::Factor),
            Not => rule(unary, None, Precedence::None),
            Asign => rule(None, None, Precedence::None),
            Equal => rule(None, binary, Precedence::Comparison),
            NotEqual => rule(None, binary, Precedence::Comparison),
            Greater => rule(None, binary, Precedence::Comparison),
            GreaterEq => rule(None, binary, Precedence::Comparison),
            Less => rule(None, binary, Precedence::Comparison),
            LessEq => rule(None, binary, Precedence::Comparison),
            Identifier => rule(variable, None, Precedence::None),
            String => rule(string, None, Precedence::None),
            Number => rule(number, None, Precedence::None),
            And => rule(None, None, Precedence::None),
            Class => rule(None, None, Precedence::None),
            Else => rule(None, None, Precedence::None),
            False => rule(literal, None, Precedence::None),
            For => rule(None, None, Precedence::None),
            Fun => rule(None, None, Precedence::None),
            If => rule(None, None, Precedence::None),
            Nil => rule(literal, None, Precedence::None),
            Or => rule(None, None, Precedence::None),
            Print => rule(None, None, Precedence::None),
            Return => rule(None, None, Precedence::None),
            Super => rule(None, None, Precedence::None),
            This => rule(None, None, Precedence::None),
            True => rule(literal, None, Precedence::None),
            Var => rule(None, None, Precedence::None),
            While => rule(None, None, Precedence::None),
            Error => rule(None, None, Precedence::None),
            Eof => rule(None, None, Precedence::None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::instruction::Instruction;
    use super::*;

    #[test]
    fn test_number_compilation() {
        assert_compile(
            "123;",
            vec![(Instruction::Const { offset: 0 }, 1), (Instruction::Pop, 1), (Instruction::Return, 1)],
        );
    }

    #[test]
    fn test_addition_compilation() {
        assert_compile(
            "1 + 2;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Add, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_multiline_compilation() {
        assert_compile(
            "1 \n + 2;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 2),
                (Instruction::Add, 2),
                (Instruction::Pop, 2),
                (Instruction::Return, 2),
            ],
        );
    }

    #[test]
    fn test_basic_arithmetic() {
        assert_compile(
            "1 - 2;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Subtract, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
        assert_compile(
            "2 * 3;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Multiply, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
        assert_compile(
            "4 / 2;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Divide, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_unary_negation() {
        assert_compile(
            "-123;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Negate, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_grouping() {
        assert_compile(
            "(123);",
            vec![(Instruction::Const { offset: 0 }, 1), (Instruction::Pop, 1), (Instruction::Return, 1)],
        );
    }

    #[test]
    fn test_precedence() {
        // 1 + 2 * 3 should be 1 + (2 * 3)
        assert_compile(
            "1 + 2 * 3;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Const { offset: 2 }, 1),
                (Instruction::Multiply, 1),
                (Instruction::Add, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );

        // (1 + 2) * 3
        assert_compile(
            "(1 + 2) * 3;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Add, 1),
                (Instruction::Const { offset: 2 }, 1),
                (Instruction::Multiply, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_associativity() {
        // 1 - 2 - 3 should be (1 - 2) - 3
        assert_compile(
            "1 - 2 - 3;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Subtract, 1),
                (Instruction::Const { offset: 2 }, 1),
                (Instruction::Subtract, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_edge_cases() {
        // Double negation --1
        assert_compile(
            "--1;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Negate, 1),
                (Instruction::Negate, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );

        // Deeply nested grouping
        assert_compile(
            "(((1)));",
            vec![(Instruction::Const { offset: 0 }, 1), (Instruction::Pop, 1), (Instruction::Return, 1)],
        );

        // Negated grouping
        assert_compile(
            "-(1 + 2);",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Add, 1),
                (Instruction::Negate, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_literals() {
        assert_compile("true;", vec![(Instruction::LoadTrue, 1), (Instruction::Pop, 1), (Instruction::Return, 1)]);
        assert_compile("false;", vec![(Instruction::LoadFalse, 1), (Instruction::Pop, 1), (Instruction::Return, 1)]);
        assert_compile("nil;", vec![(Instruction::LoadNil, 1), (Instruction::Pop, 1), (Instruction::Return, 1)]);
    }

    #[test]
    fn test_comparison() {
        assert_compile(
            "1 < 2;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Less, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
        assert_compile(
            "1 <= 2;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Greater, 1),
                (Instruction::Not, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
        assert_compile(
            "1 > 2;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Greater, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
        assert_compile(
            "1 >= 2;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Less, 1),
                (Instruction::Not, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_equality() {
        assert_compile(
            "1 == 2;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Equal, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
        assert_compile(
            "1 != 2;",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Equal, 1),
                (Instruction::Not, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_logical_not() {
        assert_compile(
            "!true;",
            vec![
                (Instruction::LoadTrue, 1),
                (Instruction::Not, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
        assert_compile(
            "!!true;",
            vec![
                (Instruction::LoadTrue, 1),
                (Instruction::Not, 1),
                (Instruction::Not, 1),
                (Instruction::Pop, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_global_variables() {
        // Declaration and access
        assert_compile_program(
            r#"
              var a = 1;
              print a;
            "#,
            vec![
                (Instruction::Const { offset: 1 }, 2),
                (Instruction::DefineGlobal { index: 0 }, 2),
                (Instruction::GetGlobal { index: 2 }, 3),
                (Instruction::Print, 3),
                (Instruction::Return, 4),
            ],
        );

        // Assignment
        assert_compile_program(
            r#"
              var a = 1;
              a = 2;
            "#,
            vec![
                (Instruction::Const { offset: 1 }, 2),
                (Instruction::DefineGlobal { index: 0 }, 2),
                (Instruction::Const { offset: 3 }, 3),
                (Instruction::SetGlobal { index: 2 }, 3),
                (Instruction::Pop, 3),
                (Instruction::Return, 4),
            ],
        );
    }

    #[test]
    fn test_local_variables() {
        assert_compile_program(
            r#"
              {
                var a = 1;
                print a;
              }
            "#,
            vec![
                (Instruction::Const { offset: 0 }, 3),
                (Instruction::GetLocal { index: 0 }, 4),
                (Instruction::Print, 4),
                (Instruction::Pop, 5),
                (Instruction::Return, 6),
            ],
        );
    }

    #[test]
    fn test_shadowing() {
        assert_compile_program(
            r#"
              var a = "global";
              {
                var a = "local";
                print a;
              }
              print a;
            "#,
            vec![
                (Instruction::Const { offset: 1 }, 2),
                (Instruction::DefineGlobal { index: 0 }, 2),
                (Instruction::Const { offset: 2 }, 4),
                (Instruction::GetLocal { index: 0 }, 5),
                (Instruction::Print, 5),
                (Instruction::Pop, 6),
                (Instruction::GetGlobal { index: 3 }, 7),
                (Instruction::Print, 7),
                (Instruction::Return, 8),
            ],
        );
    }

    #[test]
    fn test_expression_statements() {
        assert_compile_program(
            r#"
              1 + 2;
              3 * 4;
            "#,
            vec![
                (Instruction::Const { offset: 0 }, 2),
                (Instruction::Const { offset: 1 }, 2),
                (Instruction::Add, 2),
                (Instruction::Pop, 2),
                (Instruction::Const { offset: 2 }, 3),
                (Instruction::Const { offset: 3 }, 3),
                (Instruction::Multiply, 3),
                (Instruction::Pop, 3),
                (Instruction::Return, 4),
            ],
        );
    }

    #[test]
    fn test_syntax_errors() {
        assert_compile_error("1 + 2", "Expected ';' after expression");
        assert_compile_error("var a = 1", "Expected ';' after variable declaration");
        assert_compile_error("print 1", "Expected ';' after expression");
    }

    #[test]
    fn test_semantic_errors() {
        // Local redeclaration
        assert_compile_error(
            r#"
              {
                var a = 1;
                var a = 2;
              }
            "#,
            "Already a variable with this name in scope",
        );

        // Self-initialization
        assert_compile_error(
            r#"
              {
                var a = a;
              }
            "#,
            "Can't read local variable in its own initializer",
        );
    }

    #[test]
    fn test_synchronization() {
        // Missing semicolon in first statement should not prevent second statement from compiling
        let source = r#"
          var a = 1
          print "synchronized";
        "#;

        let mut buffer = Vec::new();
        let mut chunk = Chunk::new();
        let has_error;
        let scanner = Scanner::new(source);
        let mut logger = Logger::new(&mut buffer);
        let mut heap = MemoryManager::new();
        {
            let mut compiler = ByteCodeCompiler::new(&scanner, &mut logger, &mut chunk, &mut heap);

            compiler.compile();
            compiler.write_end();
            has_error = compiler.has_error;
        }

        assert!(has_error);

        // Check if "synchronized" string constant was still added (meaning it reached the second statement)
        let has_synchronized = chunk.constants.iter().any(|v| v.is_string_object() && v.as_object().str() == "synchronized");
        assert!(has_synchronized, "Compiler failed to synchronize and parse subsequent statements");
    }

    fn assert_compile_error(source: &str, expected_msg: &str) {
        let mut buffer = Vec::new();
        let has_error;
        {
            let scanner = Scanner::new(source);
            let mut chunk = Chunk::new();
            let mut logger = Logger::new(&mut buffer);
            let mut heap = MemoryManager::new();
            let mut compiler = ByteCodeCompiler::new(&scanner, &mut logger, &mut chunk, &mut heap);

            compiler.compile();
            has_error = compiler.has_error;
        }

        assert!(has_error, "Expected compilation error but none was reported");
        let output = String::from_utf8_lossy(&buffer);
        assert!(
            output.contains(expected_msg),
            "Expected error message '{}' not found in output: {}",
            expected_msg,
            output
        );
    }

    fn assert_compile(source: &str, expected: Vec<(Instruction, u32)>) {
        let scanner = Scanner::new(source);
        let mut chunk = Chunk::new();
        let mut logger = Logger::new(std::io::stderr());
        let mut heap = MemoryManager::new();
        let mut compiler = ByteCodeCompiler::new(&scanner, &mut logger, &mut chunk, &mut heap);

        compiler.compile();
        compiler.write_end();

        assert!(!compiler.has_error, "Compilation failed with errors");

        std::mem::drop(compiler);
        let mut actual = Vec::new();
        let mut iter = chunk.code.iter().copied();
        let mut byte_offset = 0;
        while let Some((instr, consumed)) = Instruction::from_bytes_iter(&mut iter) {
            actual.push((instr, chunk.lines[byte_offset]));
            byte_offset += consumed;
        }

        assert_eq!(actual, expected);
    }

    fn assert_compile_program(source: &str, expected: Vec<(Instruction, u32)>) {
        let scanner = Scanner::new(source);
        let mut chunk = Chunk::new();
        let mut logger = Logger::new(std::io::stderr());
        let mut heap = MemoryManager::new();
        let mut compiler = ByteCodeCompiler::new(&scanner, &mut logger, &mut chunk, &mut heap);

        // We use a custom compile loop here to test programs until the main compile() is updated
        compiler.advance();
        while !compiler.match_current(TokenType::Eof) {
            compiler.declaration();
        }
        compiler.write_end();

        assert!(!compiler.has_error, "Compilation failed with errors");

        std::mem::drop(compiler);
        let mut actual = Vec::new();
        let mut iter = chunk.code.iter().copied();
        let mut byte_offset = 0;
        while let Some((instr, consumed)) = Instruction::from_bytes_iter(&mut iter) {
            actual.push((instr, chunk.lines[byte_offset]));
            byte_offset += consumed;
        }

        assert_eq!(actual, expected);
    }
}
