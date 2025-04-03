use std::io::{self, Read, Result, Seek};

use peekread::PeekRead;

use crate::{
    log,
    token::{Token, TokenType},
};

pub struct Scanner<R: PeekRead + Seek> {
    reader: R,
    current: u64,
    line: u64,
    has_error: bool,
}

impl<R: PeekRead + Seek> Scanner<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            current: 0,
            line: 1,
            has_error: false,
        }
    }
}

impl<R: PeekRead + Seek> From<R> for Scanner<R> {
    fn from(reader: R) -> Self {
        Scanner::<R>::new(reader)
    }
}

impl<R: PeekRead + Seek> Scanner<R> {
    pub fn has_error(&self) -> bool {
        return self.has_error;
    }
}

impl<R: PeekRead + Seek> Scanner<R> {
    pub fn next_token(&mut self) -> Result<Token> {
        loop {
            let byte = self.advance();
            if byte.is_none() {
                return Ok(Token::eof());
            }
            use TokenType::*;
            let token = match byte.unwrap()? as char {
                '(' => Token::symbol(LeftParen, "(", self.line),
                ')' => Token::symbol(RightParen, ")", self.line),
                '{' => Token::symbol(LeftBrace, "{", self.line),
                '}' => Token::symbol(RightBrace, "}", self.line),
                '+' => Token::symbol(Plus, "+", self.line),
                '-' => Token::symbol(Minus, "-", self.line),
                '.' => Token::symbol(Dot, ".", self.line),
                '*' => Token::symbol(Star, "*", self.line),
                ',' => Token::symbol(Comma, ",", self.line),
                ';' => Token::symbol(SemiColon, ";", self.line),
                '=' if self.matchup('=') => Token::symbol(Equal, "==", self.line),
                '=' => Token::symbol(Asign, "=", self.line),
                '!' if self.matchup('=') => Token::symbol(NotEqual, "!=", self.line),
                '!' => Token::symbol(Not, "!", self.line),
                '<' if self.matchup('=') => Token::symbol(LessEq, "<=", self.line),
                '<' => Token::symbol(Less, "<", self.line),
                '>' if self.matchup('=') => Token::symbol(GreaterEq, ">=", self.line),
                '>' => Token::symbol(Greater, ">", self.line),
                '/' if self.matchup('/') => {
                    self.read_line()?;
                    continue;
                }
                '/' => Token::symbol(Div, "/", self.line),
                '"' => self.string().unwrap_or_else(|_| {
                    self.has_error = true;
                    Token::eof()
                }),
                d @ '0'..='9' => return self.number(d),
                '\n' => continue,
                c @ ('a'..='z' | 'A'..='Z' | '_') => return self.identifier(c),
                c if c.is_whitespace() => continue,
                c => {
                    self.has_error = true;
                    log::error_unkown_symbol(self.line, c);
                    continue;
                }
            };
            return Ok(token);
        }
    }

    fn advance(&mut self) -> Option<Result<u8>> {
        let c = (&mut self.reader).bytes().next();
        if c.is_none() {
            return c;
        }
        self.current += 1;
        if matches!(c, Some(Ok(b'\n'))) {
            self.line += 1;
        }
        c
    }

    fn number(&mut self, first: char) -> Result<Token> {
        let mut lexeme = first.to_string();
        loop {
            match self.peek() {
                Some(d @ '0'..='9') => {
                    lexeme.push(d);
                    self.advance();
                }
                Some('.') if matches!(self.peek_offset(1), Some('0'..='9')) => {
                    lexeme.push('.');
                    self.advance();
                }
                _ => break,
            }
        }
        Ok(Token::number(lexeme, self.line))
    }

    fn identifier(&mut self, first: char) -> Result<Token> {
        let mut lexeme = first.to_string();
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_alphanumeric() || c == '_' => {
                    lexeme.push(c);
                    self.advance();
                }
                _ => break,
            }
        }
        Ok(Token::textual(lexeme, self.line))
    }

    fn string(&mut self) -> Result<Token> {
        let mut s = String::new();
        loop {
            match self.advance() {
                Some(Ok(b'"')) => break,
                Some(Ok(b)) => s.push(b as char),
                Some(Err(e)) => return Err(e),
                None => {
                    log::error(self.line, "Unterminated string.");
                    self.has_error = true;
                    return Err(std::io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Invalid Input",
                    ));
                }
            }
        }
        Ok(Token::string(s, self.line))
    }

    fn read_line(&mut self) -> Result<String> {
        let mut s = String::new();
        while let Some(b) = self.advance() {
            if matches!(b, Ok(b'\n')) {
                break;
            }
            s.push(b? as char);
        }
        Ok(s)
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
        match (&mut self.reader).peek().bytes().next() {
            None => None,
            Some(Err(_)) => None,
            Some(Ok(c)) => Some(c as char),
        }
    }

    fn peek_offset(&mut self, offset: u64) -> Option<char> {
        match (&mut self.reader)
            .peek()
            .bytes()
            .skip(offset as usize)
            .next()
        {
            None => None,
            Some(Err(_)) => None,
            Some(Ok(c)) => Some(c as char),
        }
    }
}

pub struct IntoIter<R: PeekRead + Seek> {
    reached_end: bool,
    scanner: Scanner<R>
}

impl <R: PeekRead + Seek> IntoIterator for Scanner<R> {
    type Item = Result<Token>;
    type IntoIter = crate::scanner::IntoIter<R>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { 
            reached_end: false,
            scanner: self,
        }
    }
}

impl <R: PeekRead + Seek> Iterator for IntoIter<R> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reached_end {
            return None
        }
        let token = self.scanner.next_token();
        if matches!(token, Ok(Token { token_type: TokenType::Eof, ..})) {
            self.reached_end = true;
        }
        return Some(token)
    }
}