#![feature(arbitrary_self_types_pointers)]

use clap::Parser;
use std::fs;
use std::io::{self, Write};
use std::process::exit;

mod cli;
mod interpreter;
mod scanner;
mod token;

use cli::Cli;
use interpreter::vm::result::InterpreterError;
use interpreter::vm::VirtualMachine;

fn main() {
    let args = Cli::parse();
    match &args.filename {
        Some(filename) => exec_file(filename, args.debug),
        None => repl(args.debug),
    }
}

fn repl(debug: bool) {
    let mut vm = VirtualMachine::new(debug, io::stdout());
    let mut line = String::new();
    loop {
        print!("> ");
        let _ = io::stdout().flush();
        line.clear();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                println!();
                break;
            },
            Ok(_) => {
                let _ = vm.interpret(&line);
            },
            Err(err) => {
                eprintln!("Failed to read line: {}", err);
                exit(74);
            },
        }
    }
}

fn exec_file(filename: &str, debug: bool) {
    let source = match fs::read_to_string(filename) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("Failed to read file '{}': {}", filename, err);
            exit(74);
        },
    };

    let mut vm = VirtualMachine::new(debug, io::stdout());
    if let Err(err) = vm.interpret(&source) {
        match err {
            InterpreterError::Compile => exit(65),
            InterpreterError::Runtime => exit(70),
        }
    }
}
