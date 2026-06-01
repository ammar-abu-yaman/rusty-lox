use super::instruction::{Instruction, OpCode};
use crate::token::TokenType;

use super::log::Logger;
use super::Scanner;
use crate::token::Token;

use super::mem::MemoryManager;
use super::object::Object;
use super::Chunk;
use super::Value;

pub struct ByteCodeCompiler<'a> {
    scanner: &'a Scanner,
    pub has_error: bool,
    pub panic_mode: bool,
    logger: &'a mut Logger,
    chunk: &'a mut Chunk,
    mem: &'a mut MemoryManager,
    previous: Option<Token<'a>>,
    current: Option<Token<'a>>,
}

impl<'a> ByteCodeCompiler<'a> {
    pub fn new(scanner: &'a Scanner, logger: &'a mut Logger, chunk: &'a mut Chunk, mem: &'a mut MemoryManager) -> Self {
        Self {
            scanner,
            logger,
            chunk,
            mem,
            has_error: false,
            panic_mode: false,
            previous: None,
            current: None,
        }
    }
}
impl<'a> ByteCodeCompiler<'a> {
    pub fn compile(&mut self) {
        self.advance();
        self.expression();
        self.consume(TokenType::Eof, "Expected EOF");
    }
}

impl<'a> ByteCodeCompiler<'a> {
    fn declaration(&mut self) {
        use TokenType::Var;
        if (self.match_current(Var)) {
            self.var_declaration();
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
                Var | Class | Fun | For | If | While | Return => return,
                _ => {},
            }

            self.advance();
        }
    }

    fn define_var(&mut self, global_var_index: u8) {
        self.write_bytes([OpCode::DefineGlobal as u8, global_var_index]);
    }
}

impl<'a> ByteCodeCompiler<'a> {
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

impl<'a> ByteCodeCompiler<'a> {
    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn number(&mut self, _can_assign: bool) {
        let value = Value::Number(self.previous.as_ref().unwrap().literal.as_number());
        let const_offset = self.add_const(value);
        self.write_bytes([OpCode::LoadConst as u8, const_offset as u8]);
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
        let rule = ParseRule::derive_rule(operator);
        self.parse_precedence(rule.precedence.next());
        match operator {
            TokenType::Plus => self.write_bytes([OpCode::Add]),
            TokenType::Minus => self.write_bytes([OpCode::Subtract]),
            TokenType::Star => self.write_bytes([OpCode::Multiply]),
            TokenType::Div => self.write_bytes([OpCode::Divide]),
            TokenType::Equal => self.write_bytes([OpCode::Equal]),
            TokenType::NotEqual => self.write_bytes([OpCode::Equal, OpCode::Not]),
            TokenType::Greater => self.write_bytes([OpCode::Greater]),
            TokenType::GreaterEq => self.write_bytes([OpCode::Greater, OpCode::Not]),
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
        self.chunk.constants.push(Value::Object(str_object));
    }

    fn variable(&mut self, can_assign: bool) {
        self.named_var(self.previous.clone().unwrap(), can_assign);
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let rule = ParseRule::derive_rule(self.previous.as_ref().unwrap().token_type);
        if rule.prefix.is_none() {
            self.error("Expected expression");
            return;
        }

        let can_assign = precedence <= Precedence::Assignment;
        rule.prefix.unwrap()(self, can_assign);

        while ParseRule::derive_rule(self.current.as_ref().unwrap().token_type).precedence >= precedence {
            self.advance();
            let rule = ParseRule::derive_rule(self.previous.as_ref().unwrap().token_type);
            rule.infix.unwrap()(self, can_assign);
        }

        if can_assign && self.match_current(TokenType::Asign) {
            self.error("Invalid assignment target");
        }
    }

    fn parse_var(&mut self, message: impl AsRef<str>) -> u8 {
        self.consume(TokenType::Identifier, message.as_ref());
        return self.identifier_const(self.previous.clone().as_ref().unwrap());
    }

    fn named_var(&mut self, name: Token<'a>, can_assign: bool) {
        let arg = self.identifier_const(&name);
        if can_assign && self.match_current(TokenType::Asign) {
            self.expression();
            self.write_bytes([OpCode::SetGlobal as u8, arg]);
        } else {
            self.write_bytes([OpCode::GetGlobal as u8, arg]);
        }
    }

    fn identifier_const(&mut self, token: &Token<'a>) -> u8 {
        let name_obj = self.mem.intern_string(token.lexeme);
        let name_val = Value::Object(name_obj);
        return self.add_const(name_val) as u8;
    }
}

impl<'a> ByteCodeCompiler<'a> {
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

    fn add_const(&mut self, value: Value) -> usize {
        self.chunk.constants.push(value);
        if self.chunk.constants.len() > u8::MAX.into() {
            return 0;
        }
        self.chunk.constants.len() - 1
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

impl<'a> ByteCodeCompiler<'a> {
    fn error_at(&mut self, token: &Token<'_>, message: &str) {
        self.has_error = true;
        self.logger.log_err_at(token, message);
    }

    fn error(&mut self, message: &str) {
        self.error_at(&self.previous.clone().unwrap(), message);
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

type ParseFn<'a> = fn(&mut ByteCodeCompiler<'a>, bool);

#[derive(Clone, Copy)]
struct ParseRule<'a> {
    prefix: Option<ParseFn<'a>>,
    infix: Option<ParseFn<'a>>,
    precedence: Precedence,
}

impl<'a> ParseRule<'a> {
    fn new(prefix: Option<ParseFn<'a>>, infix: Option<ParseFn<'a>>, precedence: Precedence) -> Self {
        Self { prefix, infix, precedence }
    }

    fn derive_rule(token_type: TokenType) -> ParseRule<'a> {
        use TokenType::*;
        let rule = Self::new;
        let unary: Option<ParseFn<'a>> = Some(ByteCodeCompiler::unary);
        let binary: Option<ParseFn<'a>> = Some(ByteCodeCompiler::binary);
        let grouping: Option<ParseFn<'a>> = Some(ByteCodeCompiler::grouping);
        let literal: Option<ParseFn<'a>> = Some(ByteCodeCompiler::literal);
        let number: Option<ParseFn<'a>> = Some(ByteCodeCompiler::number);
        let string: Option<ParseFn<'a>> = Some(ByteCodeCompiler::string);
        let variable: Option<ParseFn<'a>> = Some(ByteCodeCompiler::variable);

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
            Not => rule(None, None, Precedence::None),
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
    use super::*;

    #[test]
    fn test_number_compilation() {
        assert_compile("123", vec![(Instruction::Const { offset: 0 }, 1), (Instruction::Return, 1)]);
    }

    #[test]
    fn test_addition_compilation() {
        assert_compile(
            "1 + 2",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Add, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_multiline_compilation() {
        assert_compile(
            "1 \n + 2",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 2),
                (Instruction::Add, 2),
                (Instruction::Return, 2),
            ],
        );
    }

    #[test]
    fn test_basic_arithmetic() {
        assert_compile(
            "1 - 2",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Subtract, 1),
                (Instruction::Return, 1),
            ],
        );
        assert_compile(
            "2 * 3",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Multiply, 1),
                (Instruction::Return, 1),
            ],
        );
        assert_compile(
            "4 / 2",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Divide, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_unary_negation() {
        assert_compile(
            "-123",
            vec![(Instruction::Const { offset: 0 }, 1), (Instruction::Negate, 1), (Instruction::Return, 1)],
        );
    }

    #[test]
    fn test_grouping() {
        assert_compile("(123)", vec![(Instruction::Const { offset: 0 }, 1), (Instruction::Return, 1)]);
    }

    #[test]
    fn test_precedence() {
        // 1 + 2 * 3 should be 1 + (2 * 3)
        assert_compile(
            "1 + 2 * 3",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Const { offset: 2 }, 1),
                (Instruction::Multiply, 1),
                (Instruction::Add, 1),
                (Instruction::Return, 1),
            ],
        );

        // (1 + 2) * 3
        assert_compile(
            "(1 + 2) * 3",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Add, 1),
                (Instruction::Const { offset: 2 }, 1),
                (Instruction::Multiply, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_associativity() {
        // 1 - 2 - 3 should be (1 - 2) - 3
        assert_compile(
            "1 - 2 - 3",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Subtract, 1),
                (Instruction::Const { offset: 2 }, 1),
                (Instruction::Subtract, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    #[test]
    fn test_edge_cases() {
        // Double negation --1
        assert_compile(
            "--1",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Negate, 1),
                (Instruction::Negate, 1),
                (Instruction::Return, 1),
            ],
        );

        // Deeply nested grouping
        assert_compile("(((1)))", vec![(Instruction::Const { offset: 0 }, 1), (Instruction::Return, 1)]);

        // Negated grouping
        assert_compile(
            "-(1 + 2)",
            vec![
                (Instruction::Const { offset: 0 }, 1),
                (Instruction::Const { offset: 1 }, 1),
                (Instruction::Add, 1),
                (Instruction::Negate, 1),
                (Instruction::Return, 1),
            ],
        );
    }

    fn assert_compile(source: &str, expected: Vec<(Instruction, u32)>) {
        let scanner = Scanner::new(source);
        let mut chunk = Chunk::new();
        let mut logger = Logger;
        let mut heap = MemoryManager::new();
        let mut compiler = ByteCodeCompiler::new(&scanner, &mut logger, &mut chunk, &mut heap);

        compiler.compile();
        compiler.write_end();

        assert!(!compiler.has_error, "Compilation failed with errors");

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
