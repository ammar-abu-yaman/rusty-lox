use crate::interpreter::vm::instruction::OpCode;
use crate::interpreter::vm::Instruction;
use crate::token::TokenType;

use super::log::Logger;
use super::Scanner;
use crate::token::Token;

use super::Chunk;
use super::Value;

pub struct ByteCodeCompiler<'a> {
    scanner: &'a Scanner,
    pub has_error: bool,
    pub panic_mode: bool,
    logger: &'a mut Logger,
    chunk: &'a mut Chunk,
    previous: Option<Token<'a>>,
    current: Option<Token<'a>>,
}

impl<'a> ByteCodeCompiler<'a> {
    pub fn new(scanner: &'a Scanner, logger: &'a mut Logger, chunk: &'a mut Chunk) -> Self {
        Self {
            scanner,
            logger,
            chunk,
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
    fn parse(&mut self, precedence: Precedence) {
        self.advance();
        let rule = ParseRule::derive_rule(self.previous.as_ref().unwrap().token_type);
        if rule.prefix.is_none() {
            self.error("Expected expression");
            return;
        }
        rule.prefix.unwrap()(self);

        while ParseRule::derive_rule(self.current.as_ref().unwrap().token_type).precedence >= precedence {
            self.advance();
            let rule = ParseRule::derive_rule(self.previous.as_ref().unwrap().token_type);
            rule.infix.unwrap()(self);
        }
    }

    fn expression(&mut self) {
        self.parse(Precedence::Assignment);
    }

    fn number(&mut self) {
        let token = self.previous.clone().unwrap();
        let value = Value::Number(token.literal.as_number());
        let const_offset = self.add_const(value);
        let instruction = Instruction::Const { offset: const_offset as u8 };
        self.write_code(instruction, token.pos.line as u32);
    }

    fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after expression");
    }

    fn unary(&mut self) {
        let operator = self.previous.as_ref().unwrap().token_type;
        self.expression();

        match operator {
            TokenType::Minus => self.chunk.write_byte(OpCode::Negate as u8, self.previous.as_ref().unwrap().pos.line as u32),
            _ => return,
        }
    }

    fn binary(&mut self) {
        let operator = self.previous.as_ref().unwrap().token_type;
        let rule = ParseRule::derive_rule(operator);
        self.parse(rule.precedence.next());
        match operator {
            TokenType::Plus => self.chunk.write_byte(OpCode::Add as u8, self.previous.as_ref().unwrap().pos.line as u32),
            TokenType::Minus => self.chunk.write_byte(OpCode::Subtract as u8, self.previous.as_ref().unwrap().pos.line as u32),
            TokenType::Star => self.chunk.write_byte(OpCode::Multiply as u8, self.previous.as_ref().unwrap().pos.line as u32),
            TokenType::Div => self.chunk.write_byte(OpCode::Divide as u8, self.previous.as_ref().unwrap().pos.line as u32),
            _ => unreachable!(""),
        }
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

    fn write_code(&mut self, instruction: Instruction, line: u32) {
        let bytes = instruction.to_bytes();
        bytes.iter().for_each(|b| self.chunk.write_byte(*b, line));
    }

    pub fn write_end(&mut self) {
        self.chunk.write_byte(OpCode::Return as u8, self.previous.as_ref().unwrap().pos.line as u32);
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

type ParseFn<'a> = fn(&mut ByteCodeCompiler<'a>);

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
        match token_type {
            LeftParen => ParseRule::new(Some(ByteCodeCompiler::grouping), None, Precedence::None),
            RightParen => ParseRule::new(None, None, Precedence::None),
            LeftBrace => ParseRule::new(None, None, Precedence::None),
            RightBrace => ParseRule::new(None, None, Precedence::None),
            Comma => ParseRule::new(None, None, Precedence::None),
            Dot => ParseRule::new(None, None, Precedence::None),
            Minus => ParseRule::new(Some(ByteCodeCompiler::unary), Some(ByteCodeCompiler::binary), Precedence::Term),
            Plus => ParseRule::new(None, Some(ByteCodeCompiler::binary), Precedence::Term),
            SemiColon => ParseRule::new(None, None, Precedence::None),
            Div => ParseRule::new(None, Some(ByteCodeCompiler::binary), Precedence::Factor),
            Star => ParseRule::new(None, Some(ByteCodeCompiler::binary), Precedence::Factor),
            Not => ParseRule::new(None, None, Precedence::None),
            NotEqual => ParseRule::new(None, None, Precedence::None),
            Asign => ParseRule::new(None, None, Precedence::None),
            Equal => ParseRule::new(None, None, Precedence::None),
            Greater => ParseRule::new(None, None, Precedence::None),
            GreaterEq => ParseRule::new(None, None, Precedence::None),
            Less => ParseRule::new(None, None, Precedence::None),
            LessEq => ParseRule::new(None, None, Precedence::None),
            Identifier => ParseRule::new(None, None, Precedence::None),
            String => ParseRule::new(None, None, Precedence::None),
            Number => ParseRule::new(Some(ByteCodeCompiler::number), None, Precedence::None),
            And => ParseRule::new(None, None, Precedence::None),
            Class => ParseRule::new(None, None, Precedence::None),
            Else => ParseRule::new(None, None, Precedence::None),
            False => ParseRule::new(None, None, Precedence::None),
            For => ParseRule::new(None, None, Precedence::None),
            Fun => ParseRule::new(None, None, Precedence::None),
            If => ParseRule::new(None, None, Precedence::None),
            Nil => ParseRule::new(None, None, Precedence::None),
            Or => ParseRule::new(None, None, Precedence::None),
            Print => ParseRule::new(None, None, Precedence::None),
            Return => ParseRule::new(None, None, Precedence::None),
            Super => ParseRule::new(None, None, Precedence::None),
            This => ParseRule::new(None, None, Precedence::None),
            True => ParseRule::new(None, None, Precedence::None),
            Var => ParseRule::new(None, None, Precedence::None),
            While => ParseRule::new(None, None, Precedence::None),
            Error => ParseRule::new(None, None, Precedence::None),
            Eof => ParseRule::new(None, None, Precedence::None),
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
        let mut compiler = ByteCodeCompiler::new(&scanner, &mut logger, &mut chunk);

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
