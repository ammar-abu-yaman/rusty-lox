use std::io::{Read, Seek, Result};

use crate::token::{Token, TokenType};

pub struct Scanner<R: Read + Seek> {
    reader: R,
    current: u64,
    line: u64,

}

impl <R: Read + Seek> Scanner<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            current: 0,
            line: 0,
        }
    }
}

impl <R: Read + Seek> From<R> for Scanner<R> {
    fn from(reader: R) -> Self {
        Scanner::<R>::new(reader)
    }
}

impl <R: Read + Seek> Scanner<R> {
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
                '=' if self.equals('=') => token(Equal, "==", self.line),
                '=' => token(Equal, "==", self.line),
                '\n' => {
                    self.line += 1;
                    continue;
                },
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

    pub fn equals(&mut self, c: char) -> bool {
        matches!(
            (&mut self.reader).bytes().peekable().peek().map(|res| res.as_ref().cloned().map(|b| b as char))
        , Some(Ok(c)))
    }

    // pub fn peek(&mut self) -> Option<Result<u8>> {
    //    return  self.reader.bytes().peekable().peek().map(|result| result.map(|b| b.clone()));
    // }
}


fn token(tt: TokenType, lexeme: &str, line: u64) -> Token {
    Token::new(tt, lexeme.to_string(), line)
}