use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::process::exit;

use parser::{LoxParser, RecursiveDecendantParser};
use peekread::SeekPeekReader;
use scanner::Scanner;
use token::TokenType;

mod parser;
mod syntax;
mod log;
mod scanner;
mod token;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        writeln!(io::stderr(), "Usage: {} tokenize <filename>", args[0]).unwrap();
        return Ok(());
    }

    let command = &args[1];
    let filename = &args[2];

    match command.as_str() {
        "tokenize" => tokenize(filename)?,
        "parse" => parse(filename)?,
        _ => {
            writeln!(io::stderr(), "Unknown command: {}", command).unwrap();
            return Ok(());
        },
    }

    return Ok(());
}

fn tokenize(filename: &str) -> Result<(), io::Error> {
    let file = File::open(filename)?;
    let mut scanner = Scanner::new(SeekPeekReader::new(file));
    let mut tokens = vec![];
    loop {
        let token = scanner.next_token()?;
        tokens.push(token);
        if tokens.last().unwrap().token_type == TokenType::Eof {
            break;
        }
    }
    tokens.iter().for_each(log::token);
    if scanner.has_error() {
        exit(65);
    }
    Ok(())
}

fn parse(filename: &str) -> Result<(), io::Error> {
    let file = File::open(filename)?;
    let scanner = Scanner::new(SeekPeekReader::new(file));
    let mut parser = RecursiveDecendantParser::new();

    let ast = parser.parse(scanner).unwrap();
    println!("{}", ast.root);

    Ok(())
}