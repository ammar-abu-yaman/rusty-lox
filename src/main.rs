#[macro_use]
extern crate num_derive;

use clap::Parser;
use std::fs::File;
use std::io::Read;
use std::process::exit;

mod cli;
mod interpreter;
mod log;
mod parser;
mod resolver;
mod scanner;
mod syntax;
mod token;

use cli::Cli;

use crate::interpreter::vm::VirtualMachine;

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    let args = Cli::parse_from(&raw_args);
    if raw_args.len() <= 1 {
        repl();
    } else if let Some(filename) = &args.filename {
        exec_file(filename);
    } else {
        eprintln!("Usage: clox [path]");
        exit(64);
    }
}

fn repl() {
    let mut vm = VirtualMachine::new(false, std::io::stdout());
    loop {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        vm.interpret(&line);
    }
}
fn exec_file(filename: &str) {
    let mut vm = VirtualMachine::new(false, std::io::stdout());
    let mut file = File::open(filename);
    if let Err(file) = file {
        eprintln!("Failed to open file: {}", file);
        exit(74);
    }
    let mut source = String::new();
    file.unwrap().read_to_string(&mut source).unwrap();
    vm.interpret(&source);
}
