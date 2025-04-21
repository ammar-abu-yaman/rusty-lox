use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::process::exit;

use interpreter::{Evaluator, Interpreter};
use parser::{LoxParser, RecursiveDecendantParser};
use scanner::Scanner;
use token::TokenType;

mod interpreter;
mod log;
mod parser;
mod scanner;
mod syntax;
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
        "evaluate" => evaluate(filename)?,
        "run" => run(filename)?,
        _ => {
            writeln!(io::stderr(), "Unknown command: {}", command).unwrap();
            return Ok(());
        }
    }

    return Ok(());
}

fn tokenize(filename: &str) -> Result<(), io::Error> {
    let file = File::open(filename)?;
    let mut scanner = Scanner::try_from(file)?;
    let mut tokens = vec![];
    loop {
        let token = scanner.next_token();
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
    let mut scanner = Scanner::try_from(file)?;
    let mut parser = RecursiveDecendantParser::new();

    let expr = parser.parse_expr(&mut scanner);
    if scanner.has_error() || expr.is_none() {
        exit(65);
    }

    println!("{}", expr.unwrap());
    Ok(())
}

fn evaluate(filename: &str) -> Result<(), io::Error> {
    let file = File::open(filename)?;
    let mut scanner = Scanner::try_from(file)?;
    let mut parser = RecursiveDecendantParser::new();

    let expr = parser.parse_expr(&mut scanner);
    if scanner.has_error() || expr.is_none() {
        exit(65);
    }

    
    let mut interpreter = interpreter::TreeWalk::new();
    let value = interpreter.eval(&expr.unwrap());
    match value {
        Ok(v) => println!("{}", v),
        Err(e) => {
            log::error_runtime(&e);
            exit(70);
        }
    }

    Ok(())
}

fn run(filename: &str) -> Result<(), io::Error> {
    let file = File::open(filename)?;
    let mut scanner = Scanner::try_from(file)?;
    let mut parser = RecursiveDecendantParser::new();

    let ast = parser.parse(&mut scanner);
    if scanner.has_error() || ast.is_none() {
        exit(65);
    }

    let mut interpreter = interpreter::TreeWalk::new();

    match interpreter.interpret(ast.unwrap()) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error_runtime(&e);
            exit(70);
        }
    }
}
