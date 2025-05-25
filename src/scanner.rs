use core::str;
use std::fs::File;
use std::io::{self, Read};

use crate::log;
use crate::token::{Token, TokenType};

pub struct Scanner {
    source: Vec<u8>,
    current: usize,
    line: u64,
    has_error: bool,
}

impl Scanner {
    pub fn new(source: Vec<u8>) -> Self {
        Self {
            source,
            current: 0,
            line: 1,
            has_error: false,
        }
    }
}

impl TryFrom<File> for Scanner {
    type Error = io::Error;

    fn try_from(mut file: File) -> io::Result<Self> {
        let mut source = Vec::new();
        file.read_to_end(&mut source)?;
        Ok(Scanner::new(source))
    }
}

impl Scanner {
    pub fn has_error(&self) -> bool {
        return self.has_error;
    }
}

impl Scanner {
    pub fn scan_all(&mut self) -> Vec<Token> {
        let mut tokens = vec![];
        loop {
            let token = self.next_token();
            tokens.push(token);
            if tokens.last().unwrap().token_type == TokenType::Eof {
                break;
            }
        }
        tokens
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            let line = self.line;
            let offset = self.current as u64;
            let byte = self.advance();
            if byte.is_none() {
                return Token::eof(line);
            }
            use TokenType::*;
            let token: Token = match byte.unwrap() as char {
                '(' => Token::symbol(LeftParen, "(", line, offset),
                ')' => Token::symbol(RightParen, ")", line, offset),
                '{' => Token::symbol(LeftBrace, "{", line, offset),
                '}' => Token::symbol(RightBrace, "}", line, offset),
                '+' => Token::symbol(Plus, "+", line, offset),
                '-' => Token::symbol(Minus, "-", line, offset),
                '.' => Token::symbol(Dot, ".", line, offset),
                '*' => Token::symbol(Star, "*", line, offset),
                ',' => Token::symbol(Comma, ",", line, offset),
                ';' => Token::symbol(SemiColon, ";", line, offset),
                '=' if self.matchup('=') => Token::symbol(Equal, "==", line, offset),
                '=' => Token::symbol(Asign, "=", line, offset),
                '!' if self.matchup('=') => Token::symbol(NotEqual, "!=", line, offset),
                '!' => Token::symbol(Not, "!", line, offset),
                '<' if self.matchup('=') => Token::symbol(LessEq, "<=", line, offset),
                '<' => Token::symbol(Less, "<", line, offset),
                '>' if self.matchup('=') => Token::symbol(GreaterEq, ">=", line, offset),
                '>' => Token::symbol(Greater, ">", line, offset),
                '/' if self.matchup('/') => {
                    self.skip_line();
                    continue;
                },
                '/' => Token::symbol(Div, "/", line, offset),
                '"' => self.string(line, offset),
                '0'..='9' => return self.number(line, offset),
                '\n' => continue,
                'a'..='z' | 'A'..='Z' | '_' => return self.identifier(line, offset),
                c if c.is_whitespace() => continue,
                c => {
                    self.has_error = true;
                    log::error_unkown_symbol(self.line, c.to_string().as_str());
                    continue;
                },
            };
            return token;
        }
    }

    fn advance(&mut self) -> Option<u8> {
        let c = self.source.get(self.current).copied();
        if c.is_none() {
            return c;
        }
        self.current += 1;
        if matches!(c, Some(b'\n')) {
            self.line += 1;
        }
        c
    }

    fn number(&mut self, line: u64, offset: u64) -> Token {
        loop {
            match self.peek() {
                Some('0'..='9') => self.advance(),
                Some('.') if matches!(self.peek_offset(1), Some('0'..='9')) => self.advance(),
                _ => break,
            };
        }
        let lexeme = str::from_utf8(&self.source[offset as usize..self.current]).unwrap();
        Token::number(lexeme, line, offset)
    }

    fn identifier(&mut self, line: u64, offset: u64) -> Token {
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_alphanumeric() || c == '_' => self.advance(),
                _ => break,
            };
        }
        let lexeme = str::from_utf8(&self.source[offset as usize..self.current]).unwrap();
        Token::textual(lexeme, line, offset)
    }

    fn string(&mut self, line: u64, offset: u64) -> Token {
        loop {
            match self.advance() {
                Some(b'"') => break,
                Some(_) => continue,
                None => {
                    log::error(self.line, "Unterminated string.");
                    self.has_error = true;
                    return Token::eof(line);
                },
            }
        }
        let lexeme = str::from_utf8(&self.source[offset as usize..self.current]).unwrap();
        Token::string(lexeme, line, offset)
    }

    fn skip_line(&mut self) {
        while let Some(b) = self.advance() {
            if b == b'\n' {
                break;
            }
        }
    }

    fn matchup(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.peek_offset(0)
    }

    fn peek_offset(&mut self, offset: u64) -> Option<char> {
        self.source.get(self.current + offset as usize).copied().map(|c| c as char)
    }
}
