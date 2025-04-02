use std::{io::{Read, Result, Seek}, result};

use peekread::PeekRead;

use crate::token::{Token, TokenType};

pub struct Scanner<R: PeekRead + Seek> {
    reader: R,
    current: u64,
    line: u64,

}

impl <R: PeekRead + Seek> Scanner<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            current: 0,
            line: 1,
        }
    }
}

impl <R: PeekRead + Seek> From<R> for Scanner<R> {
    fn from(reader: R) -> Self {
        Scanner::<R>::new(reader)
    }
}

impl <R: PeekRead + Seek> Scanner<R> {
    pub fn next(&mut self) -> Result<Token> {
        loop {
            let byte = self.advance();
            if byte.is_none() {
                return Ok(Token::eof());
            }
            use TokenType::*;
            let token = match byte.unwrap()? as char {
                '(' => token(LeftParen, "(", self.line),
                ')' => token(RightParen, ")", self.line),
                '{' => token(LeftBrace, "{", self.line),
                '}' => token(RightBrace, "}", self.line),
                '+' => token(Plus, "+", self.line),
                '-' => token(Minus, "-", self.line),
                '.' => token(Dot, ".", self.line),
                '*' => token(Star, "*", self.line),
                ',' => token(Comma, ",", self.line),
                ';' => token(SemiColon, ";", self.line),
                '=' if self.matchup('=') => token(Equal, "==", self.line),
                '=' => token(Asign, "=", self.line),
                '!' if self.matchup('=') => token(NotEqual, "!=", self.line),
                '!' => token(Not, "!", self.line),
                '<' if self.matchup('=') => token(LessEq, "<=", self.line),
                '<' => token(Less, "<", self.line),
                '>' if self.matchup('=') => token(GreaterEq, ">=", self.line),
                '>' => token(Greater, ">", self.line),
                '\n' => {
                    self.line += 1;
                    continue;
                },
                c if c.is_whitespace() => continue,
                c @ _ => token(Unkown, &c.to_string(), self.line)
            };
            return Ok(token);
        }

    }

    pub fn advance(&mut self) -> Option<Result<u8>> {
        let c = (&mut self.reader).bytes().next();
        self.current += 1;
        c
    }

    pub fn matchup(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn peek(&mut self) -> Option<char> {
        match (&mut self.reader).peek().bytes().next() {
            None => None,
            Some(Err(_)) => None,
            Some(Ok(c)) => Some(c as char)
        }
    }
}


fn token(tt: TokenType, lexeme: &str, line: u64) -> Token {
    Token::new(tt, lexeme.to_string(), line)
}