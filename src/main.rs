use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::{self, Write};
use std::process::exit;

use peekread::{BufPeekReader, SeekPeekReader};
use scanner::Scanner;
use token::Token;
use token::TokenType;

mod token;
mod scanner;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        writeln!(io::stderr(), "Usage: {} tokenize <filename>", args[0]).unwrap();
        return Ok(());
    }

    let command = &args[1];
    let filename = &args[2];

    match command.as_str() {
        "tokenize" => {            
            let mut has_errors = false;
            let file = File::open(filename)?;
            let mut scanner = Scanner::new(SeekPeekReader::new(file));
            loop {
                let token = scanner.next()?;
                match token.token_type {
                    TokenType::Unkown => {
                        lex_error(&token);
                        has_errors = true;
                    }
                    TokenType::Eof => {
                        println!("{}", token.to_token_string());
                        break;
                    }
                    _ => println!("{}", token.to_token_string()),
                }
            }
            if has_errors { 
                exit(65);
            }
        }
        _ => {
            writeln!(io::stderr(), "Unknown command: {}", command).unwrap();
            return Ok(());
        }
    }
    
    return Ok(());
}


fn lex_error(token: &Token) {
    eprintln!("[line {}] Error: Unexpected character: {}", token.pos.line, token.lexeme);
}